pub mod consts;
pub mod instr;
pub mod reg;
pub mod scope;

use self::consts::Consts;
use self::instr::{Instr, Instrs};
use self::reg::{RegAlloc, RegId};
use self::scope::{ScopeStack, VarLocation};
use crate::syntax::*;
use crate::Value;

#[derive(Default)]
pub struct Compiler {
    regs: RegAlloc,
    instrs: Instrs,
    consts: Consts,
    scopes: ScopeStack,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler::default()
    }

    fn push_scope(&mut self) {
        self.scopes.push();
    }

    fn pop_scope(&mut self) {
        for loc in self.scopes.pop() {
            if let VarLocation::Reg(reg) = loc {
                self.regs.free(reg)
            }
        }
    }

    pub fn compile_expr(&mut self, expr: Expr, dst: RegId) {
        match expr {
            Expr::Null(expr) => self.compile_expr_null(expr, dst),
            Expr::Bool(expr) => self.compile_expr_bool(expr, dst),
            Expr::Int(expr) => self.compile_expr_int(expr, dst),
            Expr::Float(expr) => self.compile_expr_float(expr, dst),
            Expr::String(expr) => self.compile_expr_string(expr, dst),
            Expr::Binding(expr) => self.compile_expr_binding(expr, dst),
            Expr::Binary(expr) => self.compile_expr_binary(expr, dst),
            Expr::Unary(expr) => self.compile_expr_unary(expr, dst),
            Expr::Grouped(expr) => self.compile_expr_grouped(expr, dst),
            Expr::List(expr) => self.compile_expr_list(expr, dst),
            Expr::Map(expr) => self.compile_expr_map(expr, dst),
            Expr::Call(expr) => self.compile_expr_call(expr, dst),
            Expr::Index(expr) => self.compile_expr_index(expr, dst),
            Expr::IfElse(expr) => self.compile_expr_if_else(expr, dst),
            Expr::LetIn(expr) => self.compile_expr_let_in(expr, dst),
            Expr::Match(expr) => self.compile_expr_match(expr, dst),
            Expr::Fn(expr) => self.compile_expr_fn(expr, dst),
        }
    }

    fn compile_const(&mut self, value: impl Into<Value>, dst: RegId) {
        let id = self.consts.add(value.into());
        self.instrs.add(Instr::LoadConst { id, dst });
    }

    fn compile_expr_null(&mut self, _expr: ExprNull, dst: RegId) {
        self.compile_const(Value::null(), dst)
    }

    fn compile_expr_bool(&mut self, expr: ExprBool, dst: RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_int(&mut self, expr: ExprInt, dst: RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_float(&mut self, expr: ExprFloat, dst: RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_string(&mut self, expr: ExprString, dst: RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_binding(&mut self, expr: ExprBinding, dst: RegId) {
        let ident = match expr.ident() {
            Some(v) => v,
            None => return,
        };

        let loc = self.scopes.get(&ident).unwrap(); // TODO

        match loc {
            VarLocation::Reg(src) => {
                self.instrs.add(Instr::Copy { src, dst });
            }
            VarLocation::Capture(_) => todo!(),
        }
    }

    fn compile_expr_binary(&mut self, expr: ExprBinary, dst: RegId) {
        let lhs = self.regs.alloc();
        if let Some(expr) = expr.lhs() {
            self.compile_expr(expr, lhs);
        }

        let rhs = dst;
        if let Some(expr) = expr.rhs() {
            self.compile_expr(expr, rhs);
        }

        let op = expr.op().unwrap_or(BinOp::Add);

        self.regs.free(lhs);
        self.instrs.add(Instr::BinOp { op, lhs, rhs, dst });
    }

    fn compile_expr_unary(&mut self, expr: ExprUnary, dst: RegId) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst);
        }

        let op = expr.op().unwrap_or(UnOp::Neg);
        self.instrs.add(Instr::UnOp { op, arg: dst, dst });
    }

    fn compile_expr_grouped(&mut self, expr: ExprGrouped, dst: RegId) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst)
        }
    }

    fn compile_expr_list(&mut self, expr: ExprList, dst: RegId) {
        let len = expr.exprs().count() as u16;
        let seq = self.regs.alloc_seq(len);

        for (expr, reg) in expr.exprs().zip(seq) {
            self.compile_expr(expr, reg);
        }

        self.instrs.add(Instr::NewList { seq, dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_map(&mut self, expr: ExprMap, dst: RegId) {
        let len = expr.pairs().count() as u16;
        let seq = self.regs.alloc_seq(len * 2);

        for (pair, reg) in expr.pairs().zip(seq.into_iter().step_by(2)) {
            if let Some(expr) = pair.key_expr() {
                self.compile_expr(expr, reg);
            }

            if let Some(ident) = pair.key_ident() {
                self.compile_const(ident.name(), reg);
            }

            if let Some(expr) = pair.value() {
                self.compile_expr(expr, RegId(reg.0 + 1));
            }
        }

        self.instrs.add(Instr::NewMap { seq, dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_call(&mut self, expr: ExprCall, dst: RegId) {
        let arity = expr.args().count() as u16;
        let seq = self.regs.alloc_seq(arity + 1);

        if let Some(expr) = expr.func() {
            self.compile_expr(expr, seq.base);
        }

        for (expr, reg) in expr.args().zip(seq.into_iter().skip(1)) {
            self.compile_expr(expr, reg);
        }

        self.instrs.add(Instr::Call { seq, dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_index(&mut self, _expr: ExprIndex, _dst: RegId) {
        todo!()
    }

    fn compile_expr_if_else(&mut self, expr: ExprIfElse, dst: RegId) {
        let cond = self.regs.alloc();

        if let Some(expr) = expr.cond() {
            self.compile_expr(expr, cond);
        }

        let start = self.instrs.add(Instr::Nop);
        self.regs.free(cond);

        if let Some(expr) = expr.if_false() {
            self.compile_expr(expr, dst);
        }

        let mid = self.instrs.add(Instr::Nop);

        if let Some(expr) = expr.if_true() {
            self.compile_expr(expr, dst);
        }

        let end = self.instrs.next_idx();

        let offset = mid - start;
        self.instrs.set(start, Instr::JumpIfTrue { cond, offset });

        let offset = end - mid - 1;
        self.instrs.set(mid, Instr::Jump { offset })
    }

    fn compile_expr_let_in(&mut self, expr: ExprLetIn, dst: RegId) {
        self.push_scope();

        for binding in expr.bindings() {
            let reg = self.regs.alloc();

            if let Some(expr) = binding.expr() {
                self.compile_expr(expr, reg);
            }

            if let Some(ident) = binding.ident() {
                self.scopes.set(ident, reg);
            }
        }

        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst);
        }

        self.pop_scope();
    }

    fn compile_expr_match(&mut self, _expr: ExprMatch, _dst: RegId) {
        todo!()
    }

    fn compile_expr_fn(&mut self, _expr: ExprFn, _dst: RegId) {
        todo!()
    }
}

pub fn compile(expr: Expr) {
    let mut compiler = Compiler::new();
    let reg = compiler.regs.alloc();
    compiler.compile_expr(expr, reg);

    for (c, id) in &compiler.consts.0 {
        eprintln!("{:?}: {:?}", id.0, c);
    }

    for instr in &compiler.instrs.0 {
        eprintln!("{:?}", instr);
    }
}
