use std::sync::Arc;

use crate::syntax::{Expr, Spanned};
use crate::value::Thunk;
use crate::vm::{Func, Instr};
use crate::Value;

#[derive(Default)]
pub struct Compiler<'expr> {
    args: &'expr [String],
    captures: Vec<&'expr str>,
    func: Func,
    parent: Option<Box<Compiler<'expr>>>,
    stack_len: u16,
}

impl<'expr> Compiler<'expr> {
    pub fn new(args: &[String]) -> Compiler<'_> {
        Compiler {
            args: &args,
            captures: Vec::new(),
            func: Func::new(args.len()),
            parent: None,
            stack_len: 0,
        }
    }

    fn compile(&mut self, expr: &'expr Spanned<Expr>) {
        match &expr.item {
            Expr::Int(v) => {
                let id = self.func.add_const(Value::Int(*v));
                self.func.add_instr(Instr::PushConst(id));
            }
            Expr::Float(v) => {
                let id = self.func.add_const(Value::Float(*v));
                self.func.add_instr(Instr::PushConst(id));
            }
            Expr::String(v) => {
                let id = self.func.add_const(Value::String(v.clone()));
                self.func.add_instr(Instr::PushConst(id));
            }
            Expr::List(list) => {
                for expr in &list.exprs {
                    self.compile(expr);
                }

                let len = u16::try_from(list.exprs.len()).expect("list too long");
                self.func.add_instr(Instr::NewList(len));
            }
            Expr::Func(expr) => {
                let parent = std::mem::take(self);
                let mut compiler = Compiler {
                    args: &expr.args,
                    captures: Vec::new(),
                    func: Func::new(expr.args.len()),
                    parent: Some(Box::new(parent)),
                    stack_len: 0,
                };

                compiler.compile(&expr.expr);
                *self = *compiler.parent.unwrap();

                let num_captures = compiler.func.captures.len() as u16;
                let id = self.func.add_const(Value::Func(Arc::new(compiler.func)));
                self.func.add_instr(Instr::PushConst(id));
                self.func.add_instr(Instr::NewFunc(num_captures));
            }
            Expr::Var(name) => {
                let mut scope = &mut *self;

                loop {
                    if let Some(idx) = scope.args.iter().position(|s| s == name) {
                        let pos = scope.stack_len + (idx as u16);
                        scope.func.add_instr(Instr::PushCopy(pos));
                        break;
                    } else if let Some(idx) = scope.captures.iter().position(|s| s == name) {
                        scope.func.add_instr(Instr::PushCapture(idx as u16));
                        break;
                    } else if let Some(parent) = &mut scope.parent {
                        let idx = scope.captures.len() as u16;
                        scope.func.add_instr(Instr::PushCapture(idx));
                        scope.func.captures.push(Value::Null);
                        scope.captures.push(name);
                        scope = parent;
                    } else {
                        panic!("cannot find {}", name);
                    }
                }
            }
            Expr::BinOp(expr) => {
                self.compile(&expr.lhs);
                self.compile(&expr.rhs);
                self.stack_len -= 2;
                self.func.add_instr(Instr::BinOp(expr.op));
            }
            Expr::UnOp(expr) => {
                self.compile(&expr.expr);
                self.stack_len -= 2;
                self.func.add_instr(Instr::UnOp(expr.op));
            }
            Expr::IfElse(expr) => {
                self.compile(&expr.cond);

                let start = self.func.add_instr(Instr::Nop);
                self.compile(&expr.if_false);
                let mid = self.func.add_instr(Instr::Nop);
                self.compile(&expr.if_true);
                let end = self.func.instrs.len();

                let offset = i16::try_from(mid - start).expect("jump too far");
                self.func.instrs[start] = Instr::JumpIf(offset);

                let offset = i16::try_from(end - mid - 1).expect("jump too far");
                self.func.instrs[mid] = Instr::Jump(offset);
            }
            Expr::Error => {}
        }

        self.stack_len += 1;
    }
}

pub fn compile(expr: &Spanned<Expr>) -> Value {
    match &expr.item {
        Expr::Func(func) => {
            let mut compiler = Compiler::new(&func.args);
            compiler.compile(&func.expr);
            Value::Func(Arc::new(compiler.func))
        }
        _ => {
            let mut compiler = Compiler::new(&[]);
            compiler.compile(&expr);
            Value::Thunk(Arc::new(Thunk::new(compiler.func)))
        }
    }
}
