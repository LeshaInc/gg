use std::collections::HashMap;
use std::sync::Arc;

use crate::syntax::{Expr, Spanned};
use crate::value::Thunk;
use crate::vm::{Func, Instr};
use crate::Value;

#[derive(Default)]
pub struct Compiler<'expr> {
    scope: Scope<'expr>,
    parent_scopes: Vec<Scope<'expr>>,
    parent: Option<Box<Compiler<'expr>>>,
    instrs: Vec<Instr>,
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

    fn add_instr(&mut self, instr: Instr) -> usize {
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
            Expr::Int(v) => {
                let id = self.add_const(Value::Int(*v));
                self.add_instr(Instr::PushConst(id));
            }
            Expr::Float(v) => {
                let id = self.add_const(Value::Float(*v));
                self.add_instr(Instr::PushConst(id));
            }
            Expr::String(v) => {
                let id = self.add_const(Value::String(v.clone()));
                self.add_instr(Instr::PushConst(id));
            }
            Expr::List(list) => {
                for expr in &list.exprs {
                    self.compile(expr);
                }

                let len = u16::try_from(list.exprs.len()).expect("list too long");
                self.add_instr(Instr::NewList(len));
            }
            Expr::Func(expr) => {
                let parent = std::mem::take(self);
                let mut compiler = Compiler::new(&expr.args);
                compiler.parent = Some(Box::new(parent));
                compiler.compile(&expr.expr);
                let func = compiler.finish();
                *self = *compiler.parent.unwrap();

                let id = self.add_const(Value::Func(Arc::new(func)));
                self.add_instr(Instr::PushConst(id));

                let num_captures = compiler.num_captures;
                if num_captures > 0 {
                    self.add_instr(Instr::NewFunc(num_captures));
                }
            }
            Expr::Call(expr) => {
                for arg in &expr.args {
                    self.compile(arg);
                }

                self.compile(&expr.func);
                self.add_instr(Instr::Call);

                self.stack_len -= expr.args.len() as u16 + 1;
            }
            Expr::Var(name) => {
                let mut current = &mut *self;

                loop {
                    if let Some(location) = current.scope.vars.get(&**name) {
                        match location {
                            VarLocation::Stack(idx) => {
                                let pos = current.stack_len - idx - 1;
                                current.add_instr(Instr::PushCopy(pos));
                            }
                            VarLocation::Capture(idx) => {
                                current.add_instr(Instr::PushCapture(*idx));
                            }
                        }

                        break;
                    } else if let Some(parent) = &mut current.parent {
                        let idx = current.num_captures;
                        let location = VarLocation::Capture(idx);
                        current.instrs.push(Instr::PushCapture(idx));
                        current.scope.vars.insert(&**name, location);
                        current.num_captures += 1;
                        current = parent;
                    } else {
                        panic!("cannot find {}", name);
                    }
                }
            }
            Expr::BinOp(expr) => {
                self.compile(&expr.lhs);
                self.compile(&expr.rhs);
                self.add_instr(Instr::BinOp(expr.op));
                self.stack_len -= 2;
            }
            Expr::UnOp(expr) => {
                self.compile(&expr.expr);
                self.add_instr(Instr::UnOp(expr.op));
                self.stack_len -= 1;
            }
            Expr::IfElse(expr) => {
                self.compile(&expr.cond);

                let start = self.add_instr(Instr::Nop);
                self.stack_len -= 1;
                self.compile(&expr.if_false);
                self.stack_len -= 1;
                let mid = self.add_instr(Instr::Nop);
                self.compile(&expr.if_true);
                let end = self.instrs.len();

                let offset = i16::try_from(mid - start).expect("jump too far");
                self.instrs[start] = Instr::JumpIf(offset);

                let offset = i16::try_from(end - mid - 1).expect("jump too far");
                self.instrs[mid] = Instr::Jump(offset);
            }
            Expr::LetIn(expr) => {
                self.parent_scopes.push(self.scope.clone());

                for (binding, expr) in &expr.vars {
                    let idx = self.stack_len;
                    self.compile(expr);
                    self.scope.vars.insert(&*binding, VarLocation::Stack(idx));
                }

                self.compile(&expr.expr);

                let num_vars = expr.vars.len() as u16;
                self.add_instr(Instr::PopSwap(num_vars));
                self.stack_len -= num_vars;

                self.parent_scopes.pop();
            }
            Expr::Error => {}
        }

        self.stack_len += 1;
    }

    fn finish(&mut self) -> Func {
        if self.arity > 0 {
            self.add_instr(Instr::PopSwap(self.arity));
        }

        self.add_instr(Instr::Ret);

        Func {
            instrs: self.instrs.iter().copied().collect(),
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
            Value::Func(Arc::new(func))
        }
        _ => {
            let mut compiler = Compiler::new(&[]);
            compiler.compile(&expr);
            let func = compiler.finish();
            Value::Thunk(Arc::new(Thunk::new(func)))
        }
    }
}
