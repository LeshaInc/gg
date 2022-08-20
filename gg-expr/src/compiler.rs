use std::collections::HashMap;
use std::sync::Arc;

use crate::syntax::{Expr, Span, Spanned};
use crate::{DebugInfo, Func, Instruction, Source, Thunk, Value};

#[derive(Default)]
pub struct Compiler<'expr> {
    name: Option<&'expr str>,
    inner_name: Option<&'expr str>,
    scope: Scope<'expr>,
    parent_scopes: Vec<Scope<'expr>>,
    parent: Option<Box<Compiler<'expr>>>,
    instructions: Vec<Instruction>,
    consts: Vec<Value>,
    debug_info: DebugInfo,
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
    pub fn new(source: Arc<Source>, args: &[String]) -> Compiler<'_> {
        Compiler {
            name: None,
            inner_name: None,
            scope: Scope::from_args(args),
            parent_scopes: Vec::new(),
            parent: None,
            instructions: Vec::new(),
            consts: Vec::new(),
            debug_info: DebugInfo::new(source),
            num_captures: 0,
            arity: args.len() as u16,
            stack_len: args.len() as u16,
        }
    }

    fn add_instr(&mut self, spans: Vec<Span>, instr: Instruction) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(instr);
        self.debug_info.instruction_spans.push(spans);
        idx
    }

    fn add_const(&mut self, value: Value) -> u16 {
        let idx = self.consts.len() as u16;
        self.consts.push(value);
        idx
    }

    fn compile_root(&mut self, expr: &'expr Spanned<Expr>) {
        self.debug_info.span = expr.span;
        self.compile(expr);
    }

    fn compile(&mut self, expr: &'expr Spanned<Expr>) {
        match &expr.item {
            &Expr::Int(v) => {
                let id = self.add_const(v.into());
                self.add_instr(vec![expr.span], Instruction::PushConst(id));
            }
            &Expr::Float(v) => {
                let id = self.add_const(v.into());
                self.add_instr(vec![expr.span], Instruction::PushConst(id));
            }
            Expr::String(v) => {
                let id = self.add_const((**v).clone().into());
                self.add_instr(vec![expr.span], Instruction::PushConst(id));
            }
            Expr::List(list) => {
                for expr in &list.exprs {
                    self.compile(expr);
                }

                let len = u16::try_from(list.exprs.len()).expect("list too long");
                self.add_instr(vec![expr.span], Instruction::NewList(len));
            }
            Expr::Func(func) => {
                let parent = std::mem::take(self);
                let mut compiler = Compiler::new(parent.debug_info.source.clone(), &func.args);
                compiler.name = parent.inner_name;
                compiler.debug_info.span = expr.span;
                compiler.parent = Some(Box::new(parent));
                compiler.compile(&func.expr);
                let func = compiler.finish();
                *self = *compiler.parent.unwrap();

                let id = self.add_const(func.into());
                self.add_instr(vec![expr.span], Instruction::PushConst(id));

                let num_captures = compiler.num_captures;
                if num_captures > 0 {
                    self.add_instr(vec![expr.span], Instruction::NewFunc(num_captures));
                }
            }
            Expr::Call(call) => {
                for arg in &call.args {
                    self.compile(arg);
                }

                self.compile(&call.func);
                self.add_instr(vec![expr.span, call.func.span], Instruction::Call);

                self.stack_len -= call.args.len() as u16 + 1;
            }
            Expr::Var(name) => {
                let mut depth = 0;
                let mut current = &mut *self;

                loop {
                    if let Some(location) = current.scope.vars.get(&**name) {
                        match location {
                            VarLocation::Stack(idx) => {
                                let pos = current.stack_len - idx - 1;
                                current.add_instr(vec![], Instruction::PushCopy(pos));
                            }
                            VarLocation::Capture(idx) => {
                                current.add_instr(vec![], Instruction::PushCapture(*idx));
                            }
                        }

                        current = &mut *self;

                        for _ in 0..depth {
                            let idx = current.num_captures;
                            let location = VarLocation::Capture(idx);
                            current.instructions.push(Instruction::PushCapture(idx));
                            current.scope.vars.insert(&**name, location);
                            current.num_captures += 1;
                            current = current.parent.as_mut().unwrap();
                        }

                        break;
                    } else if current.name == Some(&**name) {
                        self.add_instr(vec![], Instruction::PushFunc(depth));
                        break;
                    } else if let Some(parent) = &mut current.parent {
                        current = parent;
                        depth += 1;
                    } else {
                        panic!("cannot find {}", name);
                    }
                }
            }
            Expr::BinOp(bin_op) => {
                self.compile(&bin_op.lhs);
                self.compile(&bin_op.rhs);
                let spans = vec![expr.span, bin_op.lhs.span, bin_op.rhs.span];
                self.add_instr(spans, Instruction::BinOp(bin_op.op));
                self.stack_len -= 2;
            }
            Expr::UnOp(un_op) => {
                self.compile(&un_op.expr);
                let spans = vec![expr.span, un_op.expr.span];
                self.add_instr(spans, Instruction::UnOp(un_op.op));
                self.stack_len -= 1;
            }
            Expr::IfElse(expr) => {
                self.compile(&expr.cond);

                let start = self.add_instr(vec![], Instruction::Nop);
                self.stack_len -= 1;
                self.compile(&expr.if_false);
                self.stack_len -= 1;
                let mid = self.add_instr(vec![], Instruction::Nop);
                self.compile(&expr.if_true);
                let end = self.instructions.len();

                let offset = i16::try_from(mid - start).expect("jump too far");
                self.instructions[start] = Instruction::JumpIf(offset);

                let offset = i16::try_from(end - mid - 1).expect("jump too far");
                self.instructions[mid] = Instruction::Jump(offset);
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
                self.add_instr(vec![], Instruction::PopSwap(num_vars));
                self.stack_len -= num_vars;

                self.parent_scopes.pop();
            }
            Expr::Error => {}
        }

        self.stack_len += 1;
    }

    fn finish(&mut self) -> Func {
        if self.arity > 0 {
            self.add_instr(vec![], Instruction::PopSwap(self.arity));
        }

        self.add_instr(vec![], Instruction::Ret);

        self.debug_info.name = self.name.map(String::from);

        Func {
            instructions: self.instructions.iter().copied().collect(),
            consts: self.consts.iter().cloned().collect(),
            captures: Vec::new(),
            debug_info: Some(Arc::new(std::mem::take(&mut self.debug_info))),
        }
    }
}

pub fn compile(source: Arc<Source>, expr: &Spanned<Expr>) -> Value {
    match &expr.item {
        Expr::Func(func) => {
            let mut compiler = Compiler::new(source, &func.args);
            compiler.compile_root(&func.expr);
            let func = compiler.finish();
            func.into()
        }
        _ => {
            let mut compiler = Compiler::new(source, &[]);
            compiler.debug_info.name = Some("<thunk>".into());
            compiler.compile_root(&expr);
            let func = compiler.finish();
            Thunk::new(func.into()).into()
        }
    }
}
