use std::collections::HashMap;

use crate::syntax::{Expr, Spanned};
use crate::{Func, Instruction, Thunk, Value};

#[derive(Default)]
pub struct Compiler<'expr> {
    name: Option<&'expr str>,
    inner_name: Option<&'expr str>,
    scope: Scope<'expr>,
    parent_scopes: Vec<Scope<'expr>>,
    parent: Option<Box<Compiler<'expr>>>,
    instrs: Vec<Instruction>,
    consts: Vec<Value>,
    num_captures: u16,
    arity: u16,
    stack_len: u16,
}

#[derive(Clone, Default)]
struct Scope<'expr> {
    vars: HashMap<&'expr str, VarLocation>,
}

impl Scope<'_> {
    fn from_args(args: &[String]) -> Scope {
        Scope {
            vars: args
                .iter()
                .enumerate()
                .map(|(i, v)| (&**v, VarLocation::Stack(i as u16)))
                .collect(),
        }
    }
}

#[derive(Clone, Copy)]
enum VarLocation {
    Stack(u16),
    Capture(u16),
}

impl<'expr> Compiler<'expr> {
    pub fn new(args: &[String]) -> Compiler<'_> {
        Compiler {
            name: None,
            inner_name: None,
            scope: Scope::from_args(args),
            parent_scopes: Vec::new(),
            parent: None,
            instrs: Vec::new(),
            consts: Vec::new(),
            num_captures: 0,
            arity: args.len() as u16,
            stack_len: args.len() as u16,
        }
    }

    fn add_instr(&mut self, instr: Instruction) -> usize {
        let idx = self.instrs.len();
        self.instrs.push(instr);
        idx
    }

    fn add_const(&mut self, value: Value) -> u16 {
        let idx = self.consts.len() as u16;
        self.consts.push(value);
        idx
    }

    fn compile(&mut self, expr: &'expr Spanned<Expr>) {
        match &expr.item {
            &Expr::Int(v) => {
                let id = self.add_const(v.into());
                self.add_instr(Instruction::PushConst(id));
            }
            &Expr::Float(v) => {
                let id = self.add_const(v.into());
                self.add_instr(Instruction::PushConst(id));
            }
            Expr::String(v) => {
                let id = self.add_const((**v).clone().into());
                self.add_instr(Instruction::PushConst(id));
            }
            Expr::List(list) => {
                for expr in &list.exprs {
                    self.compile(expr);
                }

                let len = u16::try_from(list.exprs.len()).expect("list too long");
                self.add_instr(Instruction::NewList(len));
            }
            Expr::Func(expr) => {
                let parent = std::mem::take(self);
                let mut compiler = Compiler::new(&expr.args);
                compiler.name = parent.inner_name;
                compiler.parent = Some(Box::new(parent));
                compiler.compile(&expr.expr);
                let func = compiler.finish();
                *self = *compiler.parent.unwrap();

                let id = self.add_const(func.into());
                self.add_instr(Instruction::PushConst(id));

                let num_captures = compiler.num_captures;
                if num_captures > 0 {
                    self.add_instr(Instruction::NewFunc(num_captures));
                }
            }
            Expr::Call(expr) => {
                for arg in &expr.args {
                    self.compile(arg);
                }

                self.compile(&expr.func);
                self.add_instr(Instruction::Call);

                self.stack_len -= expr.args.len() as u16 + 1;
            }
            Expr::Var(name) => {
                let mut depth = 0;
                let mut current = &mut *self;

                loop {
                    if let Some(location) = current.scope.vars.get(&**name) {
                        match location {
                            VarLocation::Stack(idx) => {
                                let pos = current.stack_len - idx - 1;
                                current.add_instr(Instruction::PushCopy(pos));
                            }
                            VarLocation::Capture(idx) => {
                                current.add_instr(Instruction::PushCapture(*idx));
                            }
                        }

                        current = &mut *self;

                        for _ in 0..depth {
                            let idx = current.num_captures;
                            let location = VarLocation::Capture(idx);
                            current.instrs.push(Instruction::PushCapture(idx));
                            current.scope.vars.insert(&**name, location);
                            current.num_captures += 1;
                            current = current.parent.as_mut().unwrap();
                        }

                        break;
                    } else if current.name == Some(&**name) {
                        self.add_instr(Instruction::PushFunc(depth));
                        break;
                    } else if let Some(parent) = &mut current.parent {
                        current = parent;
                        depth += 1;
                    } else {
                        panic!("cannot find {}", name);
                    }
                }
            }
            Expr::BinOp(expr) => {
                self.compile(&expr.lhs);
                self.compile(&expr.rhs);
                self.add_instr(Instruction::BinOp(expr.op));
                self.stack_len -= 2;
            }
            Expr::UnOp(expr) => {
                self.compile(&expr.expr);
                self.add_instr(Instruction::UnOp(expr.op));
                self.stack_len -= 1;
            }
            Expr::IfElse(expr) => {
                self.compile(&expr.cond);

                let start = self.add_instr(Instruction::Nop);
                self.stack_len -= 1;
                self.compile(&expr.if_false);
                self.stack_len -= 1;
                let mid = self.add_instr(Instruction::Nop);
                self.compile(&expr.if_true);
                let end = self.instrs.len();

                let offset = i16::try_from(mid - start).expect("jump too far");
                self.instrs[start] = Instruction::JumpIf(offset);

                let offset = i16::try_from(end - mid - 1).expect("jump too far");
                self.instrs[mid] = Instruction::Jump(offset);
            }
            Expr::LetIn(expr) => {
                self.parent_scopes.push(self.scope.clone());

                for (binding, expr) in &expr.vars {
                    let idx = self.stack_len;
                    self.inner_name = Some(&*binding);
                    self.compile(expr);
                    self.inner_name = None;
                    self.scope.vars.insert(&*binding, VarLocation::Stack(idx));
                }

                self.compile(&expr.expr);

                let num_vars = expr.vars.len() as u16;
                self.add_instr(Instruction::PopSwap(num_vars));
                self.stack_len -= num_vars;

                self.parent_scopes.pop();
            }
            Expr::Error => {}
        }

        self.stack_len += 1;
    }

    fn finish(&mut self) -> Func {
        if self.arity > 0 {
            self.add_instr(Instruction::PopSwap(self.arity));
        }

        self.add_instr(Instruction::Ret);

        Func {
            instructions: self.instrs.iter().copied().collect(),
            consts: self.consts.iter().cloned().collect(),
            captures: Vec::new(),
        }
    }
}

pub fn compile(expr: &Spanned<Expr>) -> Value {
    match &expr.item {
        Expr::Func(func) => {
            let mut compiler = Compiler::new(&func.args);
            compiler.compile(&func.expr);
            let func = compiler.finish();
            func.into()
        }
        _ => {
            let mut compiler = Compiler::new(&[]);
            compiler.compile(&expr);
            let func = compiler.finish();
            Thunk::new(func.into()).into()
        }
    }
}
