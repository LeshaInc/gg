mod reg_alloc;
mod scope;

use std::collections::HashSet;
use std::fmt::Write;
use std::iter;
use std::sync::Arc;

use self::reg_alloc::RegAlloc;
use self::scope::ScopeStack;
use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::*;
use crate::vm::*;
use crate::{DebugInfo, Func, Source, Value};

pub struct Compiler {
    regs: RegAlloc,
    instrs: Instrs,
    consts: Consts,
    scopes: ScopeStack,
    diagnostics: Vec<Diagnostic>,
    debug_info: DebugInfo,
    arity: u16,
}

impl Compiler {
    pub fn new(source: Arc<Source>) -> Compiler {
        Compiler {
            regs: Default::default(),
            instrs: Default::default(),
            consts: Default::default(),
            scopes: Default::default(),
            diagnostics: Default::default(),
            debug_info: DebugInfo::new(source),
            arity: 0,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push();
    }

    fn pop_scope(&mut self) {
        for reg in self.scopes.pop() {
            self.regs.free(reg)
        }
    }

    fn add_error(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn add_simple_error(&mut self, range: TextRange, message: &str, label: &str) {
        self.add_error(Diagnostic::new(Severity::Error, message).with_source(
            SourceComponent::new(self.debug_info.source.clone()).with_label(
                Severity::Error,
                range,
                label,
            ),
        ))
    }

    fn add_instr_ranged(&mut self, ranges: &[TextRange], instr: Instr) -> InstrIdx {
        let idx = self.instrs.add(instr);
        self.debug_info
            .instruction_ranges
            .insert(idx, ranges.into());
        idx
    }

    fn compile_const(&mut self, range: TextRange, value: impl Into<Value>, dst: RegId) {
        let src = self.consts.add(value.into());
        let instr = Instr::new(Opcode::LoadConst)
            .with_const_id(src)
            .with_reg_b(dst);
        self.add_instr_ranged(&[range], instr);
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

    fn compile_expr_dst(&mut self, expr: Expr, dst: RegId) {
        let mut tmp = dst;
        self.compile_expr(expr, &mut tmp);
        if dst != tmp {
            let instr = Instr::new(Opcode::Copy).with_reg_a(tmp).with_reg_b(dst);
            self.instrs.add(instr);
        }
    }

    fn compile_expr_null(&mut self, expr: ExprNull, dst: &mut RegId) {
        self.compile_const(expr.range(), Value::null(), *dst)
    }

    fn compile_expr_bool(&mut self, expr: ExprBool, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(expr.range(), value, *dst)
    }

    fn compile_expr_int(&mut self, expr: ExprInt, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(expr.range(), value, *dst)
    }

    fn compile_expr_float(&mut self, expr: ExprFloat, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(expr.range(), value, *dst)
    }

    fn compile_expr_string(&mut self, expr: ExprString, dst: &mut RegId) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(expr.range(), value, *dst)
    }

    fn compile_expr_binding(&mut self, expr: ExprBinding, dst: &mut RegId) {
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
        let source = self.debug_info.source.clone();
        let source =
            SourceComponent::new(source).with_label(Severity::Error, range, "no such binding");
        let mut diagnostic = Diagnostic::new(Severity::Error, message).with_source(source);

        if !in_scope.is_empty() {
            diagnostic = diagnostic.with_help(help);
        }

        self.add_error(diagnostic);
    }

    fn compile_expr_binary(&mut self, expr: ExprBinary, dst: &mut RegId) {
        let range = expr.range();
        let mut lhs_range = range;
        let mut lhs = *dst;

        if let Some(expr) = expr.lhs() {
            lhs_range = expr.range();
            self.compile_expr(expr, &mut lhs);
        }

        let mut rhs_range = range;
        let mut rhs = *dst;
        let mut rhs_temp = None;
        if lhs == rhs {
            rhs = self.regs.alloc();
            rhs_temp = Some(rhs);
        }

        if let Some(expr) = expr.rhs() {
            rhs_range = expr.range();
            self.compile_expr(expr, &mut rhs);
        }

        if let Some(reg) = rhs_temp {
            self.regs.free(reg);
        }

        let opcode = match expr.op() {
            Some(BinOp::Or) => Opcode::OpOr,
            Some(BinOp::Coalesce) => Opcode::OpCoalesce,
            Some(BinOp::And) => Opcode::OpAnd,
            Some(BinOp::Lt) => Opcode::OpLt,
            Some(BinOp::Le) => Opcode::OpLe,
            Some(BinOp::Eq) => Opcode::OpEq,
            Some(BinOp::Neq) => Opcode::OpNeq,
            Some(BinOp::Ge) => Opcode::OpGe,
            Some(BinOp::Gt) => Opcode::OpGt,
            Some(BinOp::Add) => Opcode::OpAdd,
            Some(BinOp::Sub) => Opcode::OpSub,
            Some(BinOp::Mul) => Opcode::OpMul,
            Some(BinOp::Div) => Opcode::OpDiv,
            Some(BinOp::Rem) => Opcode::OpRem,
            Some(BinOp::Pow) => Opcode::OpPow,
            Some(BinOp::Index) => Opcode::OpIndex,
            Some(BinOp::IndexNullable) => Opcode::OpIndexNullable,
            _ => Opcode::OpAdd,
        };

        let instr = Instr::new(opcode)
            .with_reg_a(lhs)
            .with_reg_b(rhs)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[expr.range(), lhs_range, rhs_range], instr);
    }

    fn compile_expr_unary(&mut self, expr: ExprUnary, dst: &mut RegId) {
        let range = expr.range();
        let mut arg_range = expr.range();
        let mut arg = *dst;
        if let Some(expr) = expr.expr() {
            arg_range = expr.range();
            self.compile_expr(expr, &mut arg);
        }

        let opcode = match expr.op() {
            Some(UnOp::Neg) => Opcode::UnOpNeg,
            Some(UnOp::Not) => Opcode::UnOpNot,
            _ => Opcode::UnOpNeg,
        };

        let instr = Instr::new(opcode).with_reg_a(arg).with_reg_b(*dst);
        self.add_instr_ranged(&[range, arg_range], instr);
    }

    fn compile_expr_grouped(&mut self, expr: ExprGrouped, dst: &mut RegId) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst)
        }
    }

    fn compile_expr_list(&mut self, expr: ExprList, dst: &mut RegId) {
        let len = expr.exprs().count() as u16;
        let seq = self.regs.alloc_seq(len);

        for (expr, dst) in expr.exprs().zip(seq) {
            self.compile_expr_dst(expr, dst);
        }

        let instr = Instr::new(Opcode::NewList)
            .with_reg_seq(seq)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[expr.range()], instr);
        self.regs.free_seq(seq);
    }

    fn compile_expr_map(&mut self, expr: ExprMap, dst: &mut RegId) {
        let len = expr.pairs().count() as u16;
        let seq = self.regs.alloc_seq(len * 2);

        for (pair, dst) in expr.pairs().zip(seq.into_iter().step_by(2)) {
            if let Some(expr) = pair.key_expr() {
                self.compile_expr_dst(expr, dst);
            } else if let Some(ident) = pair.key_ident() {
                self.compile_const(ident.range(), ident.name(), dst);
            }

            if let Some(expr) = pair.value() {
                self.compile_expr_dst(expr, RegId(dst.0 + 1));
            }
        }

        let instr = Instr::new(Opcode::NewMap)
            .with_reg_seq(seq)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[expr.range()], instr);
        self.regs.free_seq(seq);
    }

    fn compile_expr_call(&mut self, expr: ExprCall, dst: &mut RegId) {
        let range = expr.range();
        let mut fn_range = range;
        let arity = expr.args().count() as u16;
        let seq = self.regs.alloc_seq(arity + 1);

        if let Some(expr) = expr.func() {
            fn_range = expr.range();
            self.compile_expr_dst(expr, seq.base);
        }

        for (expr, dst) in expr.args().zip(seq.into_iter().skip(1)) {
            self.compile_expr_dst(expr, dst);
        }

        let instr = Instr::new(Opcode::Call).with_reg_seq(seq).with_reg_c(*dst);
        self.add_instr_ranged(&[range, fn_range], instr);
        self.regs.free_seq(seq);
    }

    fn compile_expr_index(&mut self, expr: ExprIndex, dst: &mut RegId) {
        let range = expr.range();

        let mut lhs = *dst;
        let mut lhs_range = range;

        if let Some(expr) = expr.lhs() {
            lhs_range = expr.range();
            self.compile_expr(expr, &mut lhs)
        }

        let mut rhs = *dst;
        let mut rhs_temp = None;
        let mut rhs_range = range;

        if lhs == rhs {
            dbg!(rhs);
            rhs_temp = Some(rhs);
        }

        if let Some(ident) = expr.rhs_ident() {
            rhs_range = expr.range();
            self.compile_const(rhs_range, ident.name(), rhs);
        } else if let Some(expr) = expr.rhs_expr() {
            rhs_range = expr.range();
            self.compile_expr(expr, &mut rhs);
        }

        if let Some(reg) = rhs_temp {
            self.regs.free(reg);
        }

        let opcode = match expr.op() {
            Some(BinOp::Index) => Opcode::OpIndex,
            Some(BinOp::IndexNullable) => Opcode::OpIndexNullable,
            _ => Opcode::OpIndex,
        };

        let instr = Instr::new(opcode)
            .with_reg_a(lhs)
            .with_reg_b(rhs)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[range, lhs_range, rhs_range], instr);
    }

    fn compile_expr_if_else(&mut self, expr: ExprIfElse, dst: &mut RegId) {
        let mut cond = *dst;

        if let Some(expr) = expr.cond() {
            self.compile_expr(expr, &mut cond);
        }

        let start = self.instrs.add(Instr::new(Opcode::Nop));

        if let Some(expr) = expr.if_false() {
            self.compile_expr_dst(expr, *dst);
        }

        let mid = self.instrs.add(Instr::new(Opcode::Nop));

        if let Some(expr) = expr.if_true() {
            self.compile_expr_dst(expr, *dst);
        }

        let end = self.instrs.next_idx();

        let offset = mid - start;
        let instr = Instr::new(Opcode::JumpIfTrue)
            .with_reg_a(cond)
            .with_offset(offset);
        self.instrs.set(start, instr);

        let offset = end - mid - 1;
        let instr = Instr::new(Opcode::Jump).with_offset(offset);
        self.instrs.set(mid, instr);
    }

    fn compile_expr_let_in(&mut self, expr: ExprLetIn, dst: &mut RegId) {
        self.push_scope();

        for binding in expr.bindings() {
            let tmp_reg = self.regs.alloc();
            let mut loc = tmp_reg;

            if let Some(expr) = binding.expr() {
                if let Expr::Fn(v) = expr {
                    self.compile_expr_fn_named(v, &mut loc, binding.ident());
                } else {
                    self.compile_expr(expr, &mut loc);
                }
            }

            if loc != tmp_reg {
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

    fn compile_expr_when(&mut self, expr: ExprWhen, dst: &mut RegId) {
        let src_tmp = self.regs.alloc();
        let mut src = src_tmp;
        let cond_tmp = self.regs.alloc();
        let mut cond = cond_tmp;

        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, &mut src);
        }

        let mut holes = Vec::new();

        for case in expr.cases() {
            self.push_scope();

            if let Some(pat) = case.pat() {
                cond = cond_tmp;
                self.compile_pat(pat.clone(), src, &mut cond);
            }

            let jump_idx = self.instrs.add(Instr::new(Opcode::Nop));
            let start_idx = self.instrs.next_idx();

            if let Some(expr) = case.expr() {
                self.compile_expr_dst(expr, *dst);
            }

            holes.push(self.instrs.add(Instr::new(Opcode::Nop)));

            let end_idx = self.instrs.next_idx();
            let offset = end_idx - start_idx;

            let instr = Instr::new(Opcode::JumpIfFalse)
                .with_reg_a(cond)
                .with_offset(offset);
            self.instrs.set(jump_idx, instr);

            self.pop_scope();
        }

        let end_idx = self.instrs.add(Instr::new(Opcode::Panic));

        for hole in holes {
            let offset = end_idx - hole;
            let instr = Instr::new(Opcode::Jump).with_offset(offset);
            self.instrs.set(hole, instr);
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
        self.arity = num_args;
        self.regs.advance(num_args);
    }

    fn compile_fn(&mut self, args: impl Iterator<Item = Ident>, body: Expr) {
        self.compile_args(args);
        let mut dst = self.regs.alloc();
        self.compile_expr(body, &mut dst);
        self.instrs.add(Instr::new(Opcode::Ret).with_reg_a(dst));
    }

    fn compile_expr_fn(&mut self, expr: ExprFn, dst: &mut RegId) {
        self.compile_expr_fn_named(expr, dst, None);
    }

    fn compile_expr_fn_named(&mut self, expr: ExprFn, dst: &mut RegId, name: Option<Ident>) {
        let mut compiler = Compiler::new(self.debug_info.source.clone());
        compiler.debug_info.name = Some(
            name.map(|v| v.name().into())
                .unwrap_or_else(|| "<anon>".into()),
        );
        compiler.debug_info.range = expr.range();

        if let Some(body) = expr.expr() {
            compiler.compile_fn(expr.args(), body);
        }

        let mut res = compiler.finish();
        self.diagnostics.append(&mut res.diagnostics);
        self.compile_const(expr.range(), res.func, *dst)
    }

    fn compile_pat(&mut self, pat: Pat, src: RegId, dst: &mut RegId) {
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

    fn compile_pat_grouped(&mut self, pat: PatGrouped, src: RegId, dst: &mut RegId) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat, src, dst);
        }
    }

    fn compile_pat_or(&mut self, _pat: PatOr, _src: RegId, _dst: &mut RegId) {
        todo!()
    }

    fn compile_pat_list(&mut self, _pat: PatList, _src: RegId, _dst: &mut RegId) {
        todo!()
    }

    fn compile_pat_const_eq(
        &mut self,
        range: TextRange,
        value: impl Into<Value>,
        src: RegId,
        dst: &mut RegId,
    ) {
        let lhs = *dst;
        self.compile_const(range, value, lhs);
        let instr = Instr::new(Opcode::OpEq)
            .with_reg_a(lhs)
            .with_reg_b(src)
            .with_reg_c(*dst);
        self.instrs.add(instr);
    }

    fn compile_pat_int(&mut self, pat: PatInt, src: RegId, dst: &mut RegId) {
        if let Some(value) = pat.value() {
            self.compile_pat_const_eq(pat.range(), value, src, dst);
        }
    }

    fn compile_pat_string(&mut self, pat: PatString, src: RegId, dst: &mut RegId) {
        if let Some(value) = pat.value() {
            self.compile_pat_const_eq(pat.range(), value, src, dst);
        }
    }

    fn compile_pat_rest(&mut self, pat: PatRest, _src: RegId, _dst: &mut RegId) {
        self.add_simple_error(
            pat.range(),
            "invalid pattern",
            "`...` invalid in this position",
        );
    }

    fn compile_pat_hole(&mut self, pat: PatHole, _src: RegId, dst: &mut RegId) {
        self.compile_const(pat.range(), true, *dst)
    }

    fn compile_pat_binding(&mut self, pat: PatBinding, src: RegId, dst: &mut RegId) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat, src, dst);
        } else {
            self.compile_const(pat.range(), true, *dst)
        }

        if let Some(ident) = pat.ident() {
            self.scopes.set(ident, *dst);
        }
    }

    fn finish(self) -> CompileResult {
        CompileResult {
            func: Func {
                arity: self.arity,
                slots: self.regs.slots(),
                instrs: self.instrs.compile(),
                consts: self.consts.compile(),
                debug_info: Some(Arc::new(self.debug_info)),
            },
            diagnostics: self.diagnostics,
        }
    }
}

pub fn compile(source: Arc<Source>, expr: Expr) -> CompileResult {
    let mut compiler = Compiler::new(source);
    compiler.debug_info.name = Some("<main>".into());
    compiler.debug_info.range = expr.range();
    compiler.compile_fn(iter::empty(), expr);
    compiler.finish()
}

#[derive(Debug, Clone)]
pub struct CompileResult {
    pub func: Func,
    pub diagnostics: Vec<Diagnostic>,
}
