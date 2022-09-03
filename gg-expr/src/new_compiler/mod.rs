pub mod consts;
pub mod instr;
pub mod reg;
pub mod scope;

use std::collections::HashSet;
use std::fmt::Write;
use std::sync::Arc;

use self::consts::Consts;
use self::instr::{Instr, Instrs};
use self::reg::{RegAlloc, RegId};
use self::scope::{ScopeStack, VarLocation};
use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::*;
use crate::{Source, Value};

pub struct Compiler {
    regs: RegAlloc,
    instrs: Instrs,
    consts: Consts,
    scopes: ScopeStack,
    diagnostics: Vec<Diagnostic>,
    source: Arc<Source>,
}

impl Compiler {
    pub fn new(source: Arc<Source>) -> Compiler {
        Compiler {
            regs: Default::default(),
            instrs: Default::default(),
            consts: Default::default(),
            scopes: Default::default(),
            diagnostics: Default::default(),
            source,
        }
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

    fn add_error(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn add_simple_error(&mut self, range: TextRange, message: &str, label: &str) {
        self.add_error(Diagnostic::new(Severity::Error, message).with_source(
            SourceComponent::new(self.source.clone()).with_label(Severity::Error, range, label),
        ))
    }

    fn compile_const(&mut self, value: impl Into<Value>, dst: RegId) {
        let id = self.consts.add(value.into());
        self.instrs.add(Instr::LoadConst { id, dst });
    }

    fn compile_expr_dst(&mut self, expr: Expr, dst: RegId) {
        let mut tmp = dst;
        self.compile_expr(expr, &mut tmp);

        if tmp != dst {
            self.instrs.add(Instr::Copy { src: tmp, dst });
        }
    }

    fn compile_expr(&mut self, expr: Expr, dst: &mut RegId) {
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
            Expr::When(expr) => self.compile_expr_when(expr, dst),
            Expr::Fn(expr) => self.compile_expr_fn(expr, dst),
        }
    }

    fn compile_expr_null(&mut self, _expr: ExprNull, dst: &mut RegId) {
        self.compile_const(Value::null(), *dst)
    }

    fn compile_expr_bool(&mut self, expr: ExprBool, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, *dst)
    }

    fn compile_expr_int(&mut self, expr: ExprInt, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, *dst)
    }

    fn compile_expr_float(&mut self, expr: ExprFloat, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, *dst)
    }

    fn compile_expr_string(&mut self, expr: ExprString, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, *dst)
    }

    fn compile_expr_binding(&mut self, expr: ExprBinding, dst: &mut RegId) {
        let ident = match expr.ident() {
            Some(v) => v,
            None => return,
        };

        let loc = match self.scopes.get(&ident) {
            Some(v) => v,
            None => return self.no_such_binding(ident),
        };

        match loc {
            VarLocation::Reg(reg) => *dst = reg,
            VarLocation::Capture(_) => todo!(),
        }
    }

    fn bindings_in_scope(&self) -> Vec<Ident> {
        let mut bindings = HashSet::new();

        for ident in self.scopes.names() {
            if !bindings.contains(&ident) {
                bindings.insert(ident);
            }
        }

        bindings.into_iter().collect()
    }

    fn no_such_binding(&mut self, ident: Ident) {
        let range = ident.range();
        let mut in_scope = self.bindings_in_scope();
        in_scope.sort_by_cached_key(|v| strsim::damerau_levenshtein(v.name(), ident.name()));

        let mut help = String::from("perhaps you meant ");

        for (i, ident) in in_scope.iter().take(3).enumerate() {
            if i > 0 {
                help.push_str(", ");
            }

            let _ = write!(&mut help, "`{}`", ident.name());
        }

        let message = format!("cannot find binding `{}`", ident.name());
        let source = self.source.clone();
        let source =
            SourceComponent::new(source).with_label(Severity::Error, range, "no such binding");
        let mut diagnostic = Diagnostic::new(Severity::Error, message).with_source(source);

        if !in_scope.is_empty() {
            diagnostic = diagnostic.with_help(help);
        }

        self.add_error(diagnostic);
    }

    fn compile_expr_binary(&mut self, expr: ExprBinary, dst: &mut RegId) {
        let mut lhs = *dst;

        if let Some(expr) = expr.lhs() {
            self.compile_expr(expr, &mut lhs);
        }

        let mut rhs = *dst;
        let mut rhs_tmp = None;
        if lhs == rhs {
            rhs = self.regs.alloc();
            rhs_tmp = Some(rhs);
        }

        let mut rhs = *dst;
        if let Some(expr) = expr.rhs() {
            self.compile_expr(expr, &mut rhs);
        }

        self.instrs.add(Instr::BinOp {
            op: expr.op().unwrap_or(BinOp::Add),
            lhs,
            rhs,
            dst: *dst,
        });

        if let Some(tmp) = rhs_tmp {
            self.regs.free(tmp);
        }
    }

    fn compile_expr_unary(&mut self, expr: ExprUnary, dst: &mut RegId) {
        let mut arg = *dst;
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, &mut arg);
        }

        let op = expr.op().unwrap_or(UnOp::Neg);
        self.instrs.add(Instr::UnOp { op, arg, dst: *dst });
    }

    fn compile_expr_grouped(&mut self, expr: ExprGrouped, dst: &mut RegId) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst)
        }
    }

    fn compile_expr_list(&mut self, expr: ExprList, dst: &mut RegId) {
        let len = expr.exprs().count() as u16;
        let seq = self.regs.alloc_seq(len);

        for (expr, reg) in expr.exprs().zip(seq) {
            self.compile_expr_dst(expr, reg);
        }

        self.instrs.add(Instr::NewList { seq, dst: *dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_map(&mut self, expr: ExprMap, dst: &mut RegId) {
        let len = expr.pairs().count() as u16;
        let seq = self.regs.alloc_seq(len * 2);

        for (pair, reg) in expr.pairs().zip(seq.into_iter().step_by(2)) {
            if let Some(expr) = pair.key_expr() {
                self.compile_expr_dst(expr, reg);
            } else if let Some(ident) = pair.key_ident() {
                self.compile_const(ident.name(), reg);
            }

            if let Some(expr) = pair.value() {
                self.compile_expr_dst(expr, RegId(reg.0 + 1));
            }
        }

        self.instrs.add(Instr::NewMap { seq, dst: *dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_call(&mut self, expr: ExprCall, dst: &mut RegId) {
        let arity = expr.args().count() as u16;
        let seq = self.regs.alloc_seq(arity + 1);

        if let Some(expr) = expr.func() {
            self.compile_expr_dst(expr, seq.base);
        }

        for (expr, reg) in expr.args().zip(seq.into_iter().skip(1)) {
            self.compile_expr_dst(expr, reg);
        }

        self.instrs.add(Instr::Call { seq, dst: *dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_index(&mut self, _expr: ExprIndex, _dst: &mut RegId) {
        todo!()
    }

    fn compile_expr_if_else(&mut self, expr: ExprIfElse, dst: &mut RegId) {
        let cond = self.regs.alloc();

        if let Some(expr) = expr.cond() {
            self.compile_expr_dst(expr, cond);
        }

        let start = self.instrs.add(Instr::Nop);
        self.regs.free(cond);

        if let Some(expr) = expr.if_false() {
            self.compile_expr_dst(expr, *dst);
        }

        let mid = self.instrs.add(Instr::Nop);

        if let Some(expr) = expr.if_true() {
            self.compile_expr_dst(expr, *dst);
        }

        let end = self.instrs.next_idx();

        let offset = mid - start;
        self.instrs.set(start, Instr::JumpIfTrue { cond, offset });

        let offset = end - mid - 1;
        self.instrs.set(mid, Instr::Jump { offset })
    }

    fn compile_expr_let_in(&mut self, expr: ExprLetIn, dst: &mut RegId) {
        self.push_scope();

        for binding in expr.bindings() {
            let reg = self.regs.alloc();

            if let Some(expr) = binding.expr() {
                self.compile_expr_dst(expr, reg);
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

    fn compile_expr_when(&mut self, expr: ExprWhen, dst: &mut RegId) {
        let value_tmp = self.regs.alloc();
        let mut value = value_tmp;
        let cond = self.regs.alloc();

        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, &mut value);
            if value != value_tmp {
                self.regs.free(value);
            }
        }

        let mut holes = Vec::new();

        for case in expr.cases() {
            self.push_scope();

            if let Some(pat) = case.pat() {
                self.compile_pat(pat.clone(), value, cond);
            }

            let jump_idx = self.instrs.add(Instr::Nop);
            let start_idx = self.instrs.next_idx();

            if let Some(expr) = case.expr() {
                self.compile_expr_dst(expr, *dst);
            }

            holes.push(self.instrs.add(Instr::Nop));

            let end_idx = self.instrs.next_idx();
            let offset = end_idx - start_idx;

            let instr = Instr::JumpIfFalse { offset, cond };
            self.instrs.set(jump_idx, instr);

            self.pop_scope();
        }

        let end_idx = self.instrs.add(Instr::Panic);

        for hole in holes {
            let offset = end_idx - hole;
            self.instrs.set(hole, Instr::Jump { offset });
        }
    }

    fn compile_expr_fn(&mut self, _expr: ExprFn, _dst: &mut RegId) {
        todo!()
    }

    fn compile_pat(&mut self, pat: Pat, value: RegId, dst: RegId) {
        match pat {
            Pat::Grouped(pat) => self.compile_pat_grouped(pat, value, dst),
            Pat::Or(pat) => self.compile_pat_or(pat, value, dst),
            Pat::List(pat) => self.compile_pat_list(pat, value, dst),
            Pat::Int(pat) => self.compile_pat_int(pat, value, dst),
            Pat::String(pat) => self.compile_pat_string(pat, value, dst),
            Pat::Rest(pat) => self.compile_pat_rest(pat, value, dst),
            Pat::Hole(pat) => self.compile_pat_hole(pat, value, dst),
            Pat::Binding(pat) => self.compile_pat_binding(pat, value, dst),
        }
    }

    fn compile_pat_grouped(&mut self, _pat: PatGrouped, _value: RegId, _dst: RegId) {
        todo!()
    }

    fn compile_pat_or(&mut self, _pat: PatOr, _value: RegId, _dst: RegId) {
        todo!()
    }

    fn compile_pat_list(&mut self, _pat: PatList, _value: RegId, _dst: RegId) {
        todo!()
    }

    fn compile_pat_const_eq(&mut self, pat: impl Into<Value>, value: RegId, dst: RegId) {
        let rhs = self.regs.alloc();
        self.compile_const(pat, rhs);
        self.instrs.add(Instr::BinOp {
            op: BinOp::Add,
            lhs: value,
            rhs,
            dst,
        });
        self.regs.free(rhs);
    }

    fn compile_pat_int(&mut self, pat: PatInt, value: RegId, dst: RegId) {
        if let Some(literal) = pat.value() {
            self.compile_pat_const_eq(literal, value, dst);
        }
    }

    fn compile_pat_string(&mut self, pat: PatString, value: RegId, dst: RegId) {
        if let Some(literal) = pat.value() {
            self.compile_pat_const_eq(literal, value, dst);
        }
    }

    fn compile_pat_rest(&mut self, pat: PatRest, _value: RegId, _dst: RegId) {
        self.add_simple_error(
            pat.range(),
            "invalid pattern",
            "`...` invalid in this position",
        );
    }

    fn compile_pat_hole(&mut self, _pat: PatHole, _value: RegId, dst: RegId) {
        self.compile_const(true, dst)
    }

    fn compile_pat_binding(&mut self, pat: PatBinding, value: RegId, dst: RegId) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat, value, dst);
        } else {
            self.compile_const(true, dst)
        }

        if let Some(ident) = pat.ident() {
            self.scopes.set(ident, dst);
        }
    }
}

pub fn compile(source: Arc<Source>, expr: Expr) {
    let mut compiler = Compiler::new(source);
    let reg = compiler.regs.alloc();
    compiler.compile_expr_dst(expr, reg);

    for diag in &compiler.diagnostics {
        eprintln!("{}", diag);
    }

    for (c, id) in &compiler.consts.0 {
        eprintln!("{:?}: {:?}", id.0, c);
    }

    for instr in &compiler.instrs.0 {
        eprintln!("{:?}", instr);
    }
}
