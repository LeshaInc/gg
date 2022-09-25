mod reg_alloc;
mod scope;

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::iter;
use std::sync::Arc;

use self::reg_alloc::RegAlloc;
use self::scope::{ScopeStack, VarLoc};
use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::{SyntaxKind as SK, *};
use crate::vm::*;
use crate::{DebugInfo, Func, Map, Source, Value};

pub struct Compiler {
    env: Map,
    regs: RegAlloc,
    instrs: Instrs,
    consts: Consts,
    upvalues: UpvalueNames,
    scopes: ScopeStack,
    pattern_scope: HashMap<Ident, RegId>,
    sibling_pattern_scope: HashMap<Ident, RegId>,
    diagnostics: Vec<Diagnostic>,
    debug_info: DebugInfo,
    arity: u16,
    in_ret_expr: bool,
}

impl Compiler {
    pub fn new(env: Map, source: Arc<Source>) -> Compiler {
        let mut scopes = ScopeStack::default();

        for (k, v) in env.iter() {
            if let Ok(str) = k.as_string() {
                scopes.set(Ident::from(str), v.clone());
            }
        }

        Compiler {
            env,
            scopes,
            regs: Default::default(),
            instrs: Default::default(),
            consts: Default::default(),
            upvalues: Default::default(),
            pattern_scope: Default::default(),
            sibling_pattern_scope: Default::default(),
            diagnostics: Default::default(),
            debug_info: DebugInfo::new(source),
            arity: 0,
            in_ret_expr: true,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push();
    }

    fn pop_scope(&mut self) {
        for reg in self.scopes.pop() {
            if let VarLoc::Reg(reg) = reg {
                self.regs.free(reg)
            }
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

    fn compile_expr_ret(&mut self, range: TextRange, val: RegId) {
        if self.in_ret_expr {
            let instr = Instr::new(Opcode::Ret).with_reg_a(val);
            self.add_instr_ranged(&[range], instr);
        }
    }

    fn compile_const(&mut self, range: TextRange, value: impl Into<Value>, dst: RegId) {
        let src = self.consts.add(value.into());
        let instr = Instr::new(Opcode::LoadConst)
            .with_const_id(src)
            .with_reg_b(dst);
        self.add_instr_ranged(&[range], instr);
        self.compile_expr_ret(range, dst);
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
        if !self.in_ret_expr && dst != tmp {
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

    fn compile_var_dst(&mut self, ident: Ident, dst: RegId) {
        let mut tmp = dst;
        self.compile_var(ident, &mut tmp);
        if dst != tmp {
            let instr = Instr::new(Opcode::Copy).with_reg_a(tmp).with_reg_b(dst);
            self.instrs.add(instr);
        }
    }

    fn compile_var(&mut self, ident: Ident, dst: &mut RegId) {
        let range = ident.range();
        match self.scopes.get(&ident) {
            Some(VarLoc::Reg(id)) => {
                *dst = *id;
            }
            Some(VarLoc::Upvalue(id)) => {
                let instr = Instr::new(Opcode::LoadUpvalue)
                    .with_upvalue_id(*id)
                    .with_reg_b(*dst);
                self.add_instr_ranged(&[range], instr);
            }
            Some(VarLoc::PossibleUpvalue) => {
                let id = self.upvalues.add(ident.clone());
                self.scopes.set(ident.clone(), id);
                let instr = Instr::new(Opcode::LoadUpvalue)
                    .with_upvalue_id(id)
                    .with_reg_b(*dst);
                self.add_instr_ranged(&[range], instr);
            }
            Some(VarLoc::Upfn(id)) => {
                let instr = Instr::new(Opcode::LoadUpfn)
                    .with_upfn_id(*id)
                    .with_reg_b(*dst);
                self.add_instr_ranged(&[range], instr);
            }
            Some(VarLoc::Value(val)) => {
                let value = val.clone();
                self.compile_const(range, value, *dst);
            }
            None => {
                self.no_such_var(ident);
            }
        }

        self.compile_expr_ret(range, *dst);
    }

    fn vars_in_scope(&self) -> Vec<Ident> {
        let mut vars = HashSet::new();

        for ident in self.scopes.names() {
            if !vars.contains(&ident) {
                vars.insert(ident);
            }
        }

        vars.into_iter().collect()
    }

    fn no_such_var(&mut self, ident: Ident) {
        let range = ident.range();
        let mut in_scope = self.vars_in_scope();
        in_scope.sort_by_cached_key(|v| strsim::damerau_levenshtein(v.name(), ident.name()));

        let mut help = String::from("perhaps you meant ");

        for (i, ident) in in_scope.iter().take(3).enumerate() {
            if i > 0 {
                help.push_str(", ");
            }

            let _ = write!(&mut help, "`{}`", ident.name());
        }

        let message = format!("cannot find variable `{}`", ident.name());
        let source = self.debug_info.source.clone();
        let source =
            SourceComponent::new(source).with_label(Severity::Error, range, "no such variable");
        let mut diagnostic = Diagnostic::new(Severity::Error, message).with_source(source);

        if !in_scope.is_empty() {
            diagnostic = diagnostic.with_help(help);
        }

        self.add_error(diagnostic);
    }

    fn compile_expr_binding(&mut self, expr: ExprBinding, dst: &mut RegId) {
        if let Some(ident) = expr.ident() {
            self.compile_var(ident, dst)
        }
    }

    fn compile_expr_binary(&mut self, expr: ExprBinary, dst: &mut RegId) {
        if let Some(SK::TokOr | SK::TokCoalesce | SK::TokAnd) = expr.op() {
            return self.compile_expr_binary_logic(expr, dst);
        }

        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

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
            Some(SK::TokLt) => Opcode::OpLt,
            Some(SK::TokLe) => Opcode::OpLe,
            Some(SK::TokEq) => Opcode::OpEq,
            Some(SK::TokNeq) => Opcode::OpNeq,
            Some(SK::TokGe) => Opcode::OpGe,
            Some(SK::TokGt) => Opcode::OpGt,
            Some(SK::TokAdd) => Opcode::OpAdd,
            Some(SK::TokSub) => Opcode::OpSub,
            Some(SK::TokMul) => Opcode::OpMul,
            Some(SK::TokDiv) => Opcode::OpDiv,
            Some(SK::TokRem) => Opcode::OpRem,
            Some(SK::TokPow) => Opcode::OpPow,
            _ => Opcode::OpAdd,
        };

        let instr = Instr::new(opcode)
            .with_reg_a(lhs)
            .with_reg_b(rhs)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[expr.range(), lhs_range, rhs_range], instr);

        self.in_ret_expr = in_ret_expr;
        self.compile_expr_ret(range, *dst);
    }

    fn compile_expr_binary_logic(&mut self, expr: ExprBinary, dst: &mut RegId) {
        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

        let range = expr.range();
        let is_coalesce = expr.op() == Some(SK::TokCoalesce);

        let mut cond = *dst;
        if is_coalesce {
            cond = self.regs.alloc();
        }

        if let Some(expr) = expr.lhs() {
            let mut lhs = *dst;
            self.compile_expr(expr, &mut lhs);

            if is_coalesce && lhs != *dst {
                let instr = Instr::new(Opcode::Copy).with_reg_a(lhs).with_reg_b(*dst);
                self.instrs.add(instr);
            }

            let opcode = if is_coalesce {
                Opcode::IsNull
            } else {
                Opcode::IsTruthy
            };

            let instr = Instr::new(opcode).with_reg_a(lhs).with_reg_b(cond);
            self.instrs.add(instr);
        }

        let hole = self.instrs.add(Instr::new(Opcode::Nop));

        if let Some(expr) = expr.rhs() {
            let mut rhs = *dst;
            self.compile_expr(expr, &mut rhs);

            if is_coalesce && rhs != *dst {
                let instr = Instr::new(Opcode::Copy).with_reg_a(rhs).with_reg_b(*dst);
                self.instrs.add(instr);
            }

            if !is_coalesce {
                let instr = Instr::new(Opcode::IsTruthy)
                    .with_reg_a(rhs)
                    .with_reg_b(*dst);
                self.instrs.add(instr);
            }
        }

        if is_coalesce {
            self.regs.free(cond);
        }

        let end = self.instrs.last_idx();

        let opcode = if expr.op() == Some(SK::TokOr) {
            Opcode::JumpIfTrue
        } else {
            Opcode::JumpIfFalse
        };

        let instr = Instr::new(opcode).with_reg_a(cond).with_offset(end - hole);
        self.instrs.set(hole, instr);

        self.in_ret_expr = in_ret_expr;
        self.compile_expr_ret(range, *dst);
    }

    fn compile_expr_unary(&mut self, expr: ExprUnary, dst: &mut RegId) {
        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

        let range = expr.range();
        let mut arg_range = expr.range();
        let mut arg = *dst;
        if let Some(expr) = expr.expr() {
            arg_range = expr.range();
            self.compile_expr(expr, &mut arg);
        }

        let opcode = match expr.op() {
            Some(SK::TokSub) => Opcode::UnOpNeg,
            Some(SK::TokNot) => Opcode::UnOpNot,
            _ => Opcode::UnOpNeg,
        };

        let instr = Instr::new(opcode).with_reg_a(arg).with_reg_b(*dst);
        self.add_instr_ranged(&[range, arg_range], instr);

        self.in_ret_expr = in_ret_expr;
        self.compile_expr_ret(range, *dst);
    }

    fn compile_expr_grouped(&mut self, expr: ExprGrouped, dst: &mut RegId) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst)
        }
    }

    fn compile_expr_list(&mut self, expr: ExprList, dst: &mut RegId) {
        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

        let range = expr.range();

        let len = expr.exprs().count() as u16;
        let seq = self.regs.alloc_seq(len);

        for (expr, dst) in expr.exprs().zip(seq) {
            self.compile_expr_dst(expr, dst);
        }

        let instr = Instr::new(Opcode::NewList)
            .with_reg_seq(seq)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[range], instr);
        self.regs.free_seq(seq);

        self.in_ret_expr = in_ret_expr;
        self.compile_expr_ret(range, *dst);
    }

    fn compile_expr_map(&mut self, expr: ExprMap, dst: &mut RegId) {
        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

        let range = expr.range();

        let len = expr.pairs().count() as u16;
        let seq = self.regs.alloc_seq(len * 2);

        for (pair, dst) in expr.pairs().zip(seq.into_iter().step_by(2)) {
            if let Some(expr) = pair.key_expr() {
                self.compile_expr_dst(expr, dst);
            } else if let Some(ident) = pair.key_ident() {
                self.compile_const(ident.range(), ident.name(), dst);
            }

            let dst = RegId(dst.0 + 1);
            if let Some(expr) = pair.value() {
                self.compile_expr_dst(expr, dst);
            } else if let Some(ident) = pair.key_ident() {
                self.compile_var_dst(ident, dst);
            }
        }

        let instr = Instr::new(Opcode::NewMap)
            .with_reg_seq(seq)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[range], instr);
        self.regs.free_seq(seq);

        self.in_ret_expr = in_ret_expr;
        self.compile_expr_ret(range, *dst);
    }

    fn compile_expr_call(&mut self, expr: ExprCall, dst: &mut RegId) {
        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

        let range = expr.range();
        let mut ranges = vec![range];

        let arity = expr.args().count() as u16;
        let seq = self.regs.alloc_seq(arity + 1);

        if let Some(expr) = expr.func() {
            ranges.push(expr.range());
            self.compile_expr_dst(expr, seq.base);
        }

        for (expr, dst) in expr.args().zip(seq.into_iter().skip(1)) {
            ranges.push(expr.range());
            self.compile_expr_dst(expr, dst);
        }

        self.in_ret_expr = in_ret_expr;
        let instr = if self.in_ret_expr {
            Instr::new(Opcode::TailCall).with_reg_seq(seq)
        } else {
            Instr::new(Opcode::Call).with_reg_seq(seq).with_reg_c(*dst)
        };

        self.add_instr_ranged(&ranges, instr);
        self.regs.free_seq(seq);
    }

    fn compile_expr_index(&mut self, expr: ExprIndex, dst: &mut RegId) {
        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

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
            rhs = self.regs.alloc();
            rhs_temp = Some(rhs);
        }

        if let Some(ident) = expr.rhs_ident() {
            rhs_range = ident.range();
            self.compile_const(rhs_range, ident.name(), rhs);
        } else if let Some(expr) = expr.rhs_expr() {
            rhs_range = expr.range();
            self.compile_expr(expr, &mut rhs);
        }

        if let Some(reg) = rhs_temp {
            self.regs.free(reg);
        }

        let opcode = match expr.op() {
            Some(SK::TokLBracket | SK::TokDot) => Opcode::OpIndex,
            Some(SK::TokQuestionLBracket | SK::TokQuestionDot) => Opcode::OpIndexNullable,
            _ => Opcode::OpIndex,
        };

        let instr = Instr::new(opcode)
            .with_reg_a(lhs)
            .with_reg_b(rhs)
            .with_reg_c(*dst);
        self.add_instr_ranged(&[range, lhs_range, rhs_range], instr);

        self.in_ret_expr = in_ret_expr;
        self.compile_expr_ret(range, *dst);
    }

    fn compile_expr_if_else(&mut self, expr: ExprIfElse, dst: &mut RegId) {
        let mut cond = *dst;

        if let Some(expr) = expr.cond() {
            let in_ret_expr = self.in_ret_expr;
            self.in_ret_expr = false;
            self.compile_expr(expr, &mut cond);
            self.in_ret_expr = in_ret_expr;
        }

        let start = self.instrs.add(Instr::new(Opcode::Nop));

        if let Some(expr) = expr.if_false() {
            self.compile_expr_dst(expr, *dst);
        }

        let mid = if self.in_ret_expr {
            self.instrs.last_idx()
        } else {
            self.instrs.add(Instr::new(Opcode::Nop))
        };

        if let Some(expr) = expr.if_true() {
            self.compile_expr_dst(expr, *dst);
        }

        let end = self.instrs.next_idx();

        let offset = mid - start;
        let instr = Instr::new(Opcode::JumpIfTrue)
            .with_reg_a(cond)
            .with_offset(offset);
        self.instrs.set(start, instr);

        if !self.in_ret_expr {
            let offset = end - mid - 1;
            let instr = Instr::new(Opcode::Jump).with_offset(offset);
            self.instrs.set(mid, instr);
        }
    }

    fn compile_expr_let_in(&mut self, expr: ExprLetIn, dst: &mut RegId) {
        self.push_scope();

        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

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

        self.in_ret_expr = in_ret_expr;

        if let Some(expr) = expr.expr() {
            self.compile_expr(expr, dst)
        }

        self.pop_scope();
    }

    fn compile_expr_when(&mut self, expr: ExprWhen, dst: &mut RegId) {
        let src_tmp = self.regs.alloc();
        let mut src = src_tmp;
        let cond = self.regs.alloc();

        if let Some(expr) = expr.expr() {
            let in_ret_expr = self.in_ret_expr;
            self.in_ret_expr = false;
            self.compile_expr(expr, &mut src);
            self.in_ret_expr = in_ret_expr;
        }

        let mut holes = Vec::new();

        for case in expr.cases() {
            self.push_scope();

            let in_ret_expr = self.in_ret_expr;
            self.in_ret_expr = false;

            if let Some(pat) = case.pat() {
                self.compile_pat_root(pat.clone(), src, cond);
            }

            let jump_idx = self.instrs.add(Instr::new(Opcode::Nop));
            let start_idx = self.instrs.next_idx();

            self.in_ret_expr = in_ret_expr;

            if let Some(expr) = case.expr() {
                self.compile_expr_dst(expr, *dst);
            }

            if !self.in_ret_expr {
                holes.push(self.instrs.add(Instr::new(Opcode::Nop)));
            }

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
        self.regs.free(cond);
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
    }

    fn compile_expr_fn(&mut self, expr: ExprFn, dst: &mut RegId) {
        self.compile_expr_fn_named(expr, dst, None);
    }

    fn compile_expr_fn_named(&mut self, expr: ExprFn, dst: &mut RegId, name: Option<Ident>) {
        let range = expr.range();

        let mut compiler = Compiler::new(self.env.clone(), self.debug_info.source.clone());
        compiler.debug_info.range = range;
        compiler.debug_info.name = Some(
            name.clone()
                .map(|v| v.name().into())
                .unwrap_or_else(|| "<anon>".into()),
        );

        for name in self.scopes.names() {
            let loc = if let Some(VarLoc::Upfn(id)) = self.scopes.get(&name) {
                VarLoc::Upfn(UpfnId(id.0 + 1))
            } else {
                VarLoc::PossibleUpvalue
            };

            compiler.scopes.set(name, loc);
        }

        if let Some(name) = name {
            compiler.scopes.set(name, UpfnId(0));
        }

        if let Some(body) = expr.expr() {
            compiler.compile_fn(expr.args(), body);
        }

        let in_ret_expr = self.in_ret_expr;
        self.in_ret_expr = false;

        if compiler.upvalues.is_empty() {
            let mut res = compiler.finish();
            self.diagnostics.append(&mut res.diagnostics);
            self.compile_const(expr.range(), res.func, *dst)
        } else {
            let seq = self.regs.alloc_seq(compiler.upvalues.len() + 1);
            let (fn_reg, up_regs) = seq.split_first();

            for (up_name, up_reg) in compiler.upvalues.iter().zip(up_regs) {
                self.compile_var_dst(up_name.clone(), up_reg);
            }

            let mut res = compiler.finish();
            self.diagnostics.append(&mut res.diagnostics);
            self.compile_const(expr.range(), res.func, fn_reg);

            let instr = Instr::new(Opcode::NewFunc)
                .with_reg_seq(seq)
                .with_reg_c(*dst);
            self.add_instr_ranged(&[range], instr);
        }

        self.in_ret_expr = in_ret_expr;
        self.compile_expr_ret(range, *dst);
    }

    fn compile_pat_root(&mut self, pat: Pat, val: RegId, cond: RegId) {
        self.pattern_scope.clear();

        self.compile_pat(pat, val, cond);

        for (name, &loc) in self.pattern_scope.iter() {
            self.scopes.set(name.clone(), loc);
        }
    }

    fn compile_pat(&mut self, pat: Pat, val: RegId, cond: RegId) {
        match pat {
            Pat::Grouped(pat) => self.compile_pat_grouped(pat, val, cond),
            Pat::Or(pat) => self.compile_pat_or(pat, val, cond),
            Pat::List(pat) => self.compile_pat_list(pat, val, cond),
            Pat::Null(pat) => self.compile_pat_null(pat, val, cond),
            Pat::Bool(pat) => self.compile_pat_bool(pat, val, cond),
            Pat::Int(pat) => self.compile_pat_int(pat, val, cond),
            Pat::String(pat) => self.compile_pat_string(pat, val, cond),
            Pat::Rest(pat) => self.compile_pat_rest(pat, val, cond),
            Pat::Hole(pat) => self.compile_pat_hole(pat, val, cond),
            Pat::Binding(pat) => self.compile_pat_binding(pat, val, cond),
        }
    }

    fn compile_pat_grouped(&mut self, pat: PatGrouped, val: RegId, cond: RegId) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat, val, cond);
        }
    }

    fn compile_pat_or(&mut self, pat: PatOr, val: RegId, cond: RegId) {
        let mut holes = Vec::new();

        let upscope = std::mem::take(&mut self.pattern_scope);
        let mut scope = HashMap::new();
        let mut subscopes = Vec::new();

        self.sibling_pattern_scope = HashMap::new();

        for (i, pat) in pat.pats().enumerate() {
            if i > 0 {
                self.sibling_pattern_scope = subscopes.pop().unwrap();
            }

            self.compile_pat(pat, val, cond);
            holes.push(self.instrs.add(Instr::new(Opcode::Nop)));

            for (var, &loc) in &self.pattern_scope {
                scope.insert(var.clone(), loc);
            }

            if i > 0 {
                subscopes.push(std::mem::take(&mut self.sibling_pattern_scope));
            }

            subscopes.push(std::mem::take(&mut self.pattern_scope));
        }

        let end = self.instrs.last_idx();
        for hole in holes {
            let instr = Instr::new(Opcode::JumpIfTrue)
                .with_reg_a(cond)
                .with_offset(end - hole);
            self.instrs.set(hole, instr);
        }

        for var in scope.keys() {
            if subscopes.iter().all(|scope| scope.contains_key(&var)) {
                continue;
            }

            let mut src = SourceComponent::new(self.debug_info.source.clone());

            for (subscope, pat) in subscopes.iter().zip(pat.pats()) {
                if subscope.contains_key(&var) {
                    continue;
                }

                let msg = format!("doesn't bind `{}`", var.name());
                src.add_label(Severity::Error, pat.range(), msg);
            }

            src.add_label(Severity::Error, var.range(), "not in all patterns");

            let msg = format!("variable `{}` is not bound in all patterns", var.name());
            let diag = Diagnostic::new(Severity::Error, msg).with_source(src);
            self.add_error(diag);
        }

        self.pattern_scope = upscope;
        for (var, loc) in scope {
            self.pattern_scope.insert(var, loc);
        }
    }

    fn compile_pat_list(&mut self, pat: PatList, val: RegId, cond: RegId) {
        let range = pat.range();
        let mut holes = Vec::new();

        let inner_reg = self.regs.alloc();
        let len_reg = self.regs.alloc();
        let idx_reg = self.regs.alloc();

        let num_pats = pat.pats().count();
        let mut expected_len = 0;
        let mut rest_start = false;
        let mut rest_end = false;

        for (i, pat) in pat.pats().enumerate() {
            if let Pat::Rest(_) = pat {
                if i == 0 {
                    rest_start = true;
                } else if i == num_pats - 1 && !rest_start {
                    rest_end = true;
                } else {
                    self.add_simple_error(
                        pat.range(),
                        "invalid pattern",
                        "`...` invalid in this position",
                    );
                }
            } else {
                expected_len += 1;
            }
        }

        let instr = Instr::new(Opcode::IsList).with_reg_a(val).with_reg_b(cond);
        self.instrs.add(instr);
        holes.push(self.instrs.add(Instr::new(Opcode::Nop)));

        let instr = Instr::new(Opcode::Len).with_reg_a(val).with_reg_b(len_reg);
        self.instrs.add(instr);

        self.compile_const(range, expected_len, idx_reg);

        let op = if rest_start || rest_end {
            Opcode::OpGe
        } else {
            Opcode::OpEq
        };

        let instr = Instr::new(op)
            .with_reg_a(len_reg)
            .with_reg_b(idx_reg)
            .with_reg_c(cond);
        self.instrs.add(instr);

        holes.push(self.instrs.add(Instr::new(Opcode::Nop)));

        let mut idx = if rest_start { expected_len } else { 0 };

        for pat in pat.pats() {
            if let Pat::Rest(_) = pat {
                continue;
            }

            self.compile_const(range, idx, idx_reg);

            if rest_start {
                let instr = Instr::new(Opcode::OpSub)
                    .with_reg_a(len_reg)
                    .with_reg_b(idx_reg)
                    .with_reg_c(idx_reg);
                self.instrs.add(instr);
            }

            idx += if rest_start { -1 } else { 1 };

            let instr = Instr::new(Opcode::OpIndex)
                .with_reg_a(val)
                .with_reg_b(idx_reg)
                .with_reg_c(inner_reg);
            self.instrs.add(instr);

            self.compile_pat(pat, inner_reg, cond);
            holes.push(self.instrs.add(Instr::new(Opcode::Nop)));
        }

        let end = self.instrs.last_idx();
        for hole in holes {
            if end == hole {
                continue;
            }

            let instr = Instr::new(Opcode::JumpIfFalse)
                .with_reg_a(cond)
                .with_offset(end - hole);
            self.instrs.set(hole, instr);
        }

        self.regs.free(idx_reg);
        self.regs.free(len_reg);
        self.regs.free(inner_reg);
    }

    fn compile_pat_const_eq(
        &mut self,
        range: TextRange,
        value: impl Into<Value>,
        val: RegId,
        cond: RegId,
    ) {
        let lhs = cond;
        self.compile_const(range, value, lhs);
        let instr = Instr::new(Opcode::OpEq)
            .with_reg_a(lhs)
            .with_reg_b(val)
            .with_reg_c(cond);
        self.instrs.add(instr);
    }

    fn compile_pat_null(&mut self, pat: PatNull, val: RegId, cond: RegId) {
        self.compile_pat_const_eq(pat.range(), Value::null(), val, cond);
    }

    fn compile_pat_bool(&mut self, pat: PatBool, val: RegId, cond: RegId) {
        if let Some(value) = pat.value() {
            self.compile_pat_const_eq(pat.range(), value, val, cond);
        }
    }

    fn compile_pat_int(&mut self, pat: PatInt, val: RegId, cond: RegId) {
        if let Some(value) = pat.value() {
            self.compile_pat_const_eq(pat.range(), value, val, cond);
        }
    }

    fn compile_pat_string(&mut self, pat: PatString, val: RegId, cond: RegId) {
        if let Some(value) = pat.value() {
            self.compile_pat_const_eq(pat.range(), value, val, cond);
        }
    }

    fn compile_pat_rest(&mut self, pat: PatRest, _val: RegId, _cond: RegId) {
        self.add_simple_error(
            pat.range(),
            "invalid pattern",
            "`...` invalid in this position",
        );
    }

    fn compile_pat_hole(&mut self, pat: PatHole, _val: RegId, cond: RegId) {
        self.compile_const(pat.range(), true, cond)
    }

    fn compile_pat_binding(&mut self, pat: PatBinding, val: RegId, cond: RegId) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat, val, cond);
        } else {
            self.compile_const(pat.range(), true, cond)
        }

        if let Some(ident) = pat.ident() {
            let loc = if self.pattern_scope.contains_key(&ident) {
                let msg = format!(
                    "identifier `{}` is bound more than once in a pattern",
                    ident.name()
                );
                self.add_simple_error(ident.range(), &msg, "already bound");
                self.regs.alloc()
            } else if let Some(&reg) = self.sibling_pattern_scope.get(&ident) {
                reg
            } else {
                self.regs.alloc()
            };

            self.pattern_scope.insert(ident, loc);

            let instr = Instr::new(Opcode::CopyIfTrue)
                .with_reg_a(val)
                .with_reg_b(loc)
                .with_reg_c(cond);
            self.instrs.add(instr);
        }
    }

    fn finish(self) -> CompileResult {
        CompileResult {
            func: Func {
                arity: self.arity,
                slots: self.regs.slots(),
                instrs: self.instrs.compile(),
                consts: self.consts.compile(),
                upvalues: self.upvalues.compile(),
                debug_info: Some(Arc::new(self.debug_info)),
            },
            diagnostics: self.diagnostics,
        }
    }
}

pub fn compile(env: Map, source: Arc<Source>, expr: Expr) -> CompileResult {
    let mut compiler = Compiler::new(env, source);
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
