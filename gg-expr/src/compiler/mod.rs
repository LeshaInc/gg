mod location;
mod reg_alloc;
mod scope;

use std::collections::HashSet;
use std::fmt::Write;
use std::iter;
use std::sync::Arc;

use self::location::Loc;
use self::reg_alloc::RegAlloc;
use self::scope::ScopeStack;
use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::*;
use crate::vm::*;
use crate::{Func, Source, Value};

pub struct Compiler {
    regs: RegAlloc,
    instrs: Instrs<Loc>,
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
            if let Loc::Reg(reg) = loc {
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

    fn compile_const(&mut self, value: impl Into<Value>, dst: &mut Loc) {
        let id = self.consts.add(value.into());
        *dst = Loc::Const(id);
    }

    fn compile_const_dst(&mut self, value: impl Into<Value>, dst: Loc) {
        let id = self.consts.add(value.into());
        let src = Loc::Const(id);
        self.instrs.add(Instr::Copy { src, dst });
    }

    fn compile_expr(&mut self, expr: Expr, dst: &mut Loc) {
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

    fn compile_expr_dst(&mut self, expr: Expr, dst: Loc) {
        let mut tmp = dst;
        self.compile_expr(expr, &mut tmp);
        if dst != tmp {
            self.instrs.add(Instr::Copy { src: tmp, dst });
        }
    }

    fn compile_expr_null(&mut self, _expr: ExprNull, dst: &mut Loc) {
        self.compile_const(Value::null(), dst)
    }

    fn compile_expr_bool(&mut self, expr: ExprBool, dst: &mut Loc) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_int(&mut self, expr: ExprInt, dst: &mut Loc) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_float(&mut self, expr: ExprFloat, dst: &mut Loc) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_string(&mut self, expr: ExprString, dst: &mut Loc) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value, dst)
    }

    fn compile_expr_binding(&mut self, expr: ExprBinding, dst: &mut Loc) {
        let ident = match expr.ident() {
            Some(v) => v,
            None => return,
        };

        match self.scopes.get(&ident) {
            Some(loc) => *dst = loc,
            None => {
                self.no_such_binding(ident);
            }
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

    fn compile_expr_binary(&mut self, expr: ExprBinary, dst: &mut Loc) {
        let mut lhs = *dst;

        if let Some(expr) = expr.lhs() {
            self.compile_expr(expr, &mut lhs);
        }

        let mut rhs = *dst;
        let mut rhs_temp = None;
        if lhs == rhs {
            rhs = Loc::Reg(self.regs.alloc());
            rhs_temp = Some(rhs);
        }

        if let Some(expr) = expr.rhs() {
            self.compile_expr(expr, &mut rhs);
        }

        if let Some(Loc::Reg(reg)) = rhs_temp {
            self.regs.free(reg);
        }

        self.instrs.add(Instr::BinOp {
            op: expr.op().unwrap_or(BinOp::Add),
            lhs,
            rhs,
            dst: *dst,
        });
    }

    fn compile_expr_unary(&mut self, expr: ExprUnary, dst: &mut Loc) {
        let mut arg = *dst;
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, &mut arg);
        }

        let op = expr.op().unwrap_or(UnOp::Neg);
        self.instrs.add(Instr::UnOp { op, arg, dst: *dst });
    }

    fn compile_expr_grouped(&mut self, expr: ExprGrouped, dst: &mut Loc) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst)
        }
    }

    fn compile_expr_list(&mut self, expr: ExprList, dst: &mut Loc) {
        let len = expr.exprs().count() as u16;
        let seq = self.regs.alloc_seq(len);

        for (expr, dst) in expr.exprs().zip(seq) {
            self.compile_expr_dst(expr, Loc::Reg(dst));
        }

        self.instrs.add(Instr::NewList { seq, dst: *dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_map(&mut self, expr: ExprMap, dst: &mut Loc) {
        let len = expr.pairs().count() as u16;
        let seq = self.regs.alloc_seq(len * 2);

        for (pair, dst) in expr.pairs().zip(seq.into_iter().step_by(2)) {
            if let Some(expr) = pair.key_expr() {
                self.compile_expr_dst(expr, Loc::Reg(dst));
            } else if let Some(ident) = pair.key_ident() {
                self.compile_const_dst(ident.name(), Loc::Reg(dst));
            }

            if let Some(expr) = pair.value() {
                self.compile_expr_dst(expr, Loc::Reg(RegId(dst.0 + 1)));
            }
        }

        self.instrs.add(Instr::NewMap { seq, dst: *dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_call(&mut self, expr: ExprCall, dst: &mut Loc) {
        let arity = expr.args().count() as u16;
        let seq = self.regs.alloc_seq(arity + 1);

        if let Some(expr) = expr.func() {
            self.compile_expr_dst(expr, Loc::Reg(seq.base));
        }

        for (expr, dst) in expr.args().zip(seq.into_iter().skip(1)) {
            self.compile_expr_dst(expr, Loc::Reg(dst));
        }

        self.instrs.add(Instr::Call { seq, dst: *dst });
        self.regs.free_seq(seq);
    }

    fn compile_expr_index(&mut self, _expr: ExprIndex, _dst: &mut Loc) {
        todo!()
    }

    fn compile_expr_if_else(&mut self, expr: ExprIfElse, dst: &mut Loc) {
        let mut cond = *dst;

        if let Some(expr) = expr.cond() {
            self.compile_expr(expr, &mut cond);
        }

        let start = self.instrs.add(Instr::Nop);

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
        self.instrs.set(mid, Instr::Jump { offset });
    }

    fn compile_expr_let_in(&mut self, expr: ExprLetIn, dst: &mut Loc) {
        self.push_scope();

        for binding in expr.bindings() {
            let tmp_reg = self.regs.alloc();
            let mut loc = Loc::Reg(tmp_reg);

            if let Some(expr) = binding.expr() {
                self.compile_expr(expr, &mut loc);
            }

            if loc != Loc::Reg(tmp_reg) {
                self.regs.free(tmp_reg);
            }

            if let Some(ident) = binding.ident() {
                self.scopes.set(ident, loc);
            }
        }

        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst)
        }

        self.pop_scope();
    }

    fn compile_expr_when(&mut self, expr: ExprWhen, dst: &mut Loc) {
        let src_tmp = self.regs.alloc();
        let mut src = Loc::Reg(src_tmp);
        let cond_tmp = self.regs.alloc();
        let mut cond = Loc::Reg(cond_tmp);

        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, &mut src);
        }

        let mut holes = Vec::new();

        for case in expr.cases() {
            self.push_scope();

            if let Some(pat) = case.pat() {
                cond = Loc::Reg(cond_tmp);
                self.compile_pat(pat.clone(), src, &mut cond);
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

        self.regs.free(src_tmp);
        self.regs.free(cond_tmp);
    }

    fn compile_args(&mut self, args: impl Iterator<Item = Ident>) {
        let mut num_args = 0;
        for (i, arg) in args.enumerate() {
            let reg = RegId(i as u16);
            self.scopes.set(arg, reg);
            num_args += 1;
        }
        self.regs.advance(num_args);
    }

    fn compile_fn(&mut self, args: impl Iterator<Item = Ident>, body: Expr) {
        self.compile_args(args);
        let mut dst = Loc::Reg(self.regs.alloc());
        self.compile_expr(body, &mut dst);
        self.instrs.add(Instr::Ret { arg: dst });
    }

    fn compile_expr_fn(&mut self, expr: ExprFn, dst: &mut Loc) {
        let mut compiler = Compiler::new(self.source.clone());

        if let Some(body) = expr.expr() {
            compiler.compile_fn(expr.args(), body);
        }

        let mut res = compiler.finish();
        self.diagnostics.append(&mut res.diagnostics);
        self.compile_const(res.func, dst)
    }

    fn compile_pat(&mut self, pat: Pat, src: Loc, dst: &mut Loc) {
        match pat {
            Pat::Grouped(pat) => self.compile_pat_grouped(pat, src, dst),
            Pat::Or(pat) => self.compile_pat_or(pat, src, dst),
            Pat::List(pat) => self.compile_pat_list(pat, src, dst),
            Pat::Int(pat) => self.compile_pat_int(pat, src, dst),
            Pat::String(pat) => self.compile_pat_string(pat, src, dst),
            Pat::Rest(pat) => self.compile_pat_rest(pat, src, dst),
            Pat::Hole(pat) => self.compile_pat_hole(pat, src, dst),
            Pat::Binding(pat) => self.compile_pat_binding(pat, src, dst),
        }
    }

    fn compile_pat_grouped(&mut self, pat: PatGrouped, src: Loc, dst: &mut Loc) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat, src, dst);
        }
    }

    fn compile_pat_or(&mut self, _pat: PatOr, _src: Loc, _dst: &mut Loc) {
        todo!()
    }

    fn compile_pat_list(&mut self, _pat: PatList, _src: Loc, _dst: &mut Loc) {
        todo!()
    }

    fn compile_pat_const_eq(&mut self, value: impl Into<Value>, src: Loc, dst: &mut Loc) {
        let mut lhs = *dst;
        self.compile_const(value, &mut lhs);
        self.instrs.add(Instr::BinOp {
            op: BinOp::Eq,
            lhs,
            rhs: src,
            dst: *dst,
        });
    }

    fn compile_pat_int(&mut self, pat: PatInt, src: Loc, dst: &mut Loc) {
        if let Some(value) = pat.value() {
            self.compile_pat_const_eq(value, src, dst);
        }
    }

    fn compile_pat_string(&mut self, pat: PatString, src: Loc, dst: &mut Loc) {
        if let Some(value) = pat.value() {
            self.compile_pat_const_eq(value, src, dst);
        }
    }

    fn compile_pat_rest(&mut self, pat: PatRest, _src: Loc, _dst: &mut Loc) {
        self.add_simple_error(
            pat.range(),
            "invalid pattern",
            "`...` invalid in this position",
        );
    }

    fn compile_pat_hole(&mut self, _pat: PatHole, _src: Loc, dst: &mut Loc) {
        self.compile_const(true, dst)
    }

    fn compile_pat_binding(&mut self, pat: PatBinding, src: Loc, dst: &mut Loc) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat, src, dst);
        } else {
            self.compile_const(true, dst)
        }

        if let Some(ident) = pat.ident() {
            self.scopes.set(ident, *dst);
        }
    }

    fn finish(self) -> CompileResult {
        let num_consts = self.consts.len();

        let loc_mapping = |loc| match loc {
            Loc::Invalid => RegId(0),
            Loc::Reg(reg) => RegId(reg.0 + num_consts),
            Loc::Const(id) => RegId(id.0),
        };

        let seq_mapping = |seq: RegSeq| RegSeq {
            base: RegId(seq.base.0 + num_consts),
            len: seq.len,
        };

        CompileResult {
            func: Func {
                slots: self.regs.slots() + num_consts,
                instrs: self.instrs.compile(loc_mapping, seq_mapping),
                consts: self.consts.compile(),
                debug_info: None,
            },
            diagnostics: self.diagnostics,
        }
    }
}

pub fn compile(source: Arc<Source>, expr: Expr) -> CompileResult {
    let mut compiler = Compiler::new(source);
    compiler.compile_fn(iter::empty(), expr);
    compiler.finish()
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub func: Func,
    pub diagnostics: Vec<Diagnostic>,
}
