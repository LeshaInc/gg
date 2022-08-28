use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::Arc;

use crate::diagnostic::{Component, Diagnostic, Label, Severity, SourceComponent};
use crate::syntax::*;
use crate::{DebugInfo, Func, Instruction, Source, Thunk, Value};

#[derive(Default)]
pub struct Compiler {
    ident: Option<Ident>,
    inner_ident: Option<Ident>,
    scope: Scope,
    parent_scopes: Vec<Scope>,
    parent: Option<Box<Compiler>>,
    instructions: Vec<Instruction>,
    consts: HashMap<Value, u32>,
    debug_info: DebugInfo,
    diagnostics: Vec<Diagnostic>,
    num_captures: u32,
    arity: u32,
    stack_len: u32,
}

#[derive(Clone, Default)]
struct Scope {
    bindings: HashMap<Ident, Location>,
}

#[derive(Clone, Copy)]
enum Location {
    Stack(u32),
    Capture(u32),
}

impl Compiler {
    pub fn new(source: Arc<Source>) -> Compiler {
        Compiler {
            ident: None,
            inner_ident: None,
            scope: Scope::default(),
            parent_scopes: Vec::new(),
            parent: None,
            instructions: Vec::new(),
            consts: HashMap::new(),
            debug_info: DebugInfo::new(source),
            diagnostics: Vec::new(),
            num_captures: 0,
            arity: 0,
            stack_len: 0,
        }
    }

    pub fn diagnostics(&mut self) -> impl Iterator<Item = Diagnostic> + '_ {
        self.diagnostics.drain(..)
    }

    fn add_error(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
        self.instructions.push(Instruction::Panic);
    }

    fn add_simple_error(&mut self, range: TextRange, message: &str, label: &str) {
        self.add_error(Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            components: vec![Component::Source(SourceComponent {
                source: self.debug_info.source.clone(),
                labels: vec![Label {
                    severity: Severity::Error,
                    range,
                    message: label.into(),
                }],
            })],
        });
    }

    fn try_from_usize<U: Default + TryFrom<usize>>(
        &mut self,
        range: TextRange,
        message: &str,
        value: usize,
    ) -> U {
        match value.try_into() {
            Ok(v) => v,
            Err(_) => {
                let label = format!(
                    "`{}` does not fit into `{}`",
                    value,
                    std::any::type_name::<U>()
                );

                self.add_simple_error(range, message, &label);
                U::default()
            }
        }
    }

    fn add_instr(&mut self, ranges: Vec<TextRange>, instr: Instruction) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(instr);
        self.debug_info.instruction_ranges.push(ranges);
        idx
    }

    fn compile_const(&mut self, range: TextRange, value: impl Into<Value>) {
        let value = value.into();

        if let Some(&idx) = self.consts.get(&value) {
            self.add_instr(vec![range], Instruction::PushConst(idx));
            return;
        }

        let idx = self.consts.len() as u32;
        self.consts.insert(value, idx);
        self.add_instr(vec![range], Instruction::PushConst(idx));
    }

    fn compile_expr(&mut self, expr: Expr) {
        let range = expr.range();
        match expr {
            Expr::Null(e) => self.compile_expr_null(e),
            Expr::Bool(e) => self.compile_expr_bool(e),
            Expr::Int(e) => self.compile_expr_int(e),
            Expr::Float(e) => self.compile_expr_float(e),
            Expr::String(e) => self.compile_expr_string(e),
            Expr::Binding(e) => self.compile_expr_binding(e),
            Expr::Binary(e) => self.compile_expr_binary(e),
            Expr::Unary(e) => self.compile_expr_unary(e),
            Expr::Grouped(e) => self.compile_expr_grouped(e),
            Expr::List(e) => self.compile_expr_list(e),
            Expr::Map(e) => self.compile_expr_map(e),
            Expr::Call(e) => self.compile_expr_call(e),
            Expr::Index(e) => self.compile_expr_index(e),
            Expr::IfElse(e) => self.compile_expr_if_else(e),
            Expr::LetIn(e) => self.compile_expr_let_in(e),
            Expr::Match(e) => self.compile_expr_match(e),
            Expr::Fn(e) => self.compile_expr_fn(e),
        }

        if self.stack_len == u32::MAX {
            self.add_simple_error(range, "stack overflow", "reached `2**32-1` stack entries");
        } else {
            self.stack_len += 1;
        }
    }

    fn compile_expr_null(&mut self, expr: ExprNull) {
        self.compile_const(expr.range(), Value::null());
    }

    fn compile_expr_bool(&mut self, expr: ExprBool) {
        let value = expr.value().unwrap_or_default();
        self.compile_const(expr.range(), value);
    }

    fn compile_expr_int(&mut self, expr: ExprInt) {
        if let Some(value) = expr.value() {
            self.compile_const(expr.range(), value);
        }
    }

    fn compile_expr_float(&mut self, expr: ExprFloat) {
        if let Some(value) = expr.value() {
            self.compile_const(expr.range(), value);
        }
    }

    fn compile_expr_string(&mut self, expr: ExprString) {
        if let Some(value) = expr.value() {
            self.compile_const(expr.range(), value);
        }
    }

    fn compile_expr_binding(&mut self, expr: ExprBinding) {
        let mut depth = 0;
        let mut current = &mut *self;

        let ident = match expr.ident() {
            Some(v) => v,
            None => return,
        };

        loop {
            if let Some(location) = current.scope.bindings.get(&ident) {
                match location {
                    Location::Stack(idx) => {
                        let pos = current.stack_len - idx - 1;
                        current.add_instr(vec![], Instruction::PushCopy(pos));
                    }
                    Location::Capture(idx) => {
                        current.add_instr(vec![], Instruction::PushCapture(*idx));
                    }
                }

                current = &mut *self;

                for _ in 0..depth {
                    let idx = current.num_captures;
                    let location = Location::Capture(idx);
                    current.instructions.push(Instruction::PushCapture(idx));
                    current.scope.bindings.insert(ident.clone(), location);
                    current.num_captures += 1;
                    current = current.parent.as_mut().unwrap();
                }

                break;
            } else if current.ident.as_ref() == Some(&ident) {
                self.add_instr(vec![], Instruction::PushFunc(depth));
                break;
            } else if let Some(parent) = &mut current.parent {
                current = parent;
                depth += 1;
            } else {
                self.no_such_binding(expr.range(), ident);
                break;
            }
        }
    }

    fn bindings_in_scope(&self) -> Vec<Ident> {
        let mut bindings = HashSet::new();
        let mut current = self;
        loop {
            let fn_ident = current.ident.clone();
            for binding in current.scope.bindings.keys().cloned().chain(fn_ident) {
                if !bindings.contains(&binding) {
                    bindings.insert(binding);
                }
            }

            if let Some(parent) = &current.parent {
                current = parent;
            } else {
                break;
            }
        }

        bindings.into_iter().collect()
    }

    fn no_such_binding(&mut self, range: TextRange, ident: Ident) {
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

    fn compile_expr_binary(&mut self, expr: ExprBinary) {
        let lhs = match expr.lhs() {
            Some(v) => v,
            None => return,
        };

        let lhs_range = lhs.range();
        self.compile_expr(lhs);

        let rhs = match expr.rhs() {
            Some(v) => v,
            None => return,
        };

        let rhs_range = rhs.range();
        self.compile_expr(rhs);

        let ranges = vec![expr.range(), lhs_range, rhs_range];
        let op = expr.op().unwrap_or(BinOp::Add);

        self.add_instr(ranges, Instruction::BinOp(op));
        self.stack_len -= 2;
    }

    fn compile_expr_unary(&mut self, expr: ExprUnary) {
        let inner = match expr.expr() {
            Some(v) => v,
            None => return,
        };

        let inner_range = inner.range();
        let ranges = vec![expr.range(), inner_range];
        let op = expr.op().unwrap_or(UnOp::Neg);

        self.compile_expr(inner);
        self.add_instr(ranges, Instruction::UnOp(op));
        self.stack_len -= 1;
    }

    fn compile_expr_grouped(&mut self, expr: ExprGrouped) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr);
        }
    }

    fn compile_expr_list(&mut self, expr: ExprList) {
        let mut len = 0;
        for expr in expr.exprs() {
            len += 1;
            self.compile_expr(expr);
        }

        self.add_instr(vec![expr.range()], Instruction::NewList(len));
        self.stack_len -= len;
    }

    fn compile_expr_map(&mut self, expr: ExprMap) {
        self.parent_scopes.push(self.scope.clone());

        let mut len = 0;
        for pair in expr.pairs() {
            if let Some(ident) = pair.key_ident() {
                self.compile_const(ident.range(), ident.name());
                self.stack_len += 1;
            }

            if let Some(expr) = pair.key_expr() {
                self.compile_expr(expr);
            }

            if let Some(ident) = pair.key_ident() {
                let loc = Location::Stack(self.stack_len - 1);
                self.scope.bindings.insert(ident, loc);
            }

            if let Some(expr) = pair.value() {
                self.compile_expr(expr);
            }

            len += 1;
        }

        self.scope = self.parent_scopes.pop().unwrap();
        self.add_instr(vec![expr.range()], Instruction::NewMap(len));
        self.stack_len -= len * 2;
    }

    fn compile_expr_call(&mut self, expr: ExprCall) {
        let mut num_args = 0;
        for arg in expr.args() {
            num_args += 1;
            self.compile_expr(arg);
        }

        let func = match expr.func() {
            Some(v) => v,
            None => return,
        };

        let func_range = func.range();

        self.compile_expr(func);
        self.add_instr(vec![expr.range(), func_range], Instruction::Call);

        self.stack_len -= num_args;
        self.stack_len -= 1;
    }

    fn compile_expr_index(&mut self, expr: ExprIndex) {
        let lhs = match expr.lhs() {
            Some(v) => v,
            None => return,
        };

        let lhs_range = lhs.range();
        self.compile_expr(lhs);

        let rhs_range = if let Some(ident) = expr.rhs_ident() {
            let range = ident.range();
            self.compile_const(range, ident.name());
            self.stack_len += 1;
            range
        } else if let Some(expr) = expr.rhs_expr() {
            let range = expr.range();
            self.compile_expr(expr);
            range
        } else {
            return;
        };

        let op = expr.op().unwrap_or(BinOp::Index);

        let ranges = vec![expr.range(), lhs_range, rhs_range];
        self.add_instr(ranges, Instruction::BinOp(op));
        self.stack_len -= 2;
    }

    fn compile_expr_if_else(&mut self, expr: ExprIfElse) {
        let cond = match expr.cond() {
            Some(v) => v,
            None => return,
        };

        let cond_range = cond.range();
        self.compile_expr(cond);

        let start = self.add_instr(vec![expr.range(), cond_range], Instruction::Nop);
        self.stack_len -= 1;

        if let Some(expr) = expr.if_false() {
            self.compile_expr(expr);
            self.stack_len -= 1;
        }

        let mid = self.add_instr(vec![], Instruction::Nop);

        if let Some(expr) = expr.if_true() {
            self.compile_expr(expr);
        }

        let end = self.instructions.len();

        let offset = self.try_from_usize(expr.range(), "if expression too long", mid - start);
        self.instructions[start] = Instruction::JumpIfTrue(offset);

        let offset = self.try_from_usize(expr.range(), "if expression too long", end - mid - 1);
        self.instructions[mid] = Instruction::Jump(offset);
    }

    fn compile_expr_let_in(&mut self, expr: ExprLetIn) {
        self.parent_scopes.push(self.scope.clone());

        let mut num_bindings = 0;
        for binding in expr.bindings() {
            num_bindings += 1;

            let idx = self.stack_len;
            if let Some(ident) = binding.ident() {
                self.inner_ident = Some(ident);
            }

            if let Some(expr) = binding.expr() {
                self.compile_expr(expr);
            }

            if let Some(ident) = binding.ident() {
                self.inner_ident = None;
                self.scope.bindings.insert(ident, Location::Stack(idx));
            }
        }

        if let Some(expr) = expr.expr() {
            self.compile_expr(expr);
        }

        self.add_instr(vec![], Instruction::PopSwap(num_bindings));
        self.stack_len -= num_bindings;

        self.scope = self.parent_scopes.pop().unwrap();
    }

    fn compile_expr_match(&mut self, expr: ExprMatch) {
        if let Some(expr) = expr.expr() {
            self.compile_expr(expr);
        }

        let mut holes = Vec::new();

        for case in expr.cases() {
            if let Some(pat) = case.pat() {
                self.compile_pat(pat);
            }

            let jump_ip = self.add_instr(vec![], Instruction::Nop);
            let start_ip = self.instructions.len();

            if let Some(expr) = case.expr() {
                self.compile_expr(expr);
                holes.push(self.add_instr(vec![], Instruction::Nop));
            }

            let end_ip = self.instructions.len();
            let offset = (end_ip - start_ip) as i32;
            self.instructions[jump_ip] = Instruction::JumpIfFalse(offset);
        }

        self.add_instr(vec![], Instruction::Panic);
        let end_ip = self.instructions.len();

        for hole in holes {
            let offset = (end_ip - hole - 1) as i32;
            self.instructions[hole] = Instruction::Jump(offset);
        }
    }

    fn compile_expr_fn(&mut self, expr: ExprFn) {
        let range = expr.range();

        let mut parent = std::mem::take(self);
        let mut compiler = Compiler::new(parent.debug_info.source.clone());

        compiler.ident = parent.inner_ident.clone();
        compiler.debug_info.range = expr.range();
        compiler.diagnostics = std::mem::take(&mut parent.diagnostics);
        compiler.parent = Some(Box::new(parent));

        let mut num_args = 0;
        for ident in expr.args() {
            let location = Location::Stack(num_args);
            compiler.scope.bindings.insert(ident, location);

            num_args += 1;
        }

        compiler.arity = num_args;
        compiler.stack_len = num_args;

        if let Some(expr) = expr.expr() {
            compiler.compile_expr(expr);
        }

        let func = compiler.finish();
        *self = *compiler.parent.unwrap();
        self.diagnostics = compiler.diagnostics;

        self.compile_const(expr.range(), func);

        let num_captures = compiler.num_captures;
        if num_captures > 0 {
            self.add_instr(vec![range], Instruction::NewFunc(num_captures));
        }
    }

    fn compile_pat(&mut self, pat: Pat) {
        match pat {
            Pat::Grouped(pat) => self.compile_pat_grouped(pat),
            Pat::Or(pat) => self.compile_pat_or(pat),
            Pat::List(pat) => self.compile_pat_list(pat),
            Pat::Int(pat) => self.compile_pat_int(pat),
            Pat::String(pat) => self.compile_pat_string(pat),
            Pat::Rest(pat) => self.compile_pat_rest(pat),
            Pat::Hole(pat) => self.compile_pat_hole(pat),
            Pat::Binding(pat) => self.compile_pat_binding(pat),
        }
    }

    fn compile_pat_grouped(&mut self, pat: PatGrouped) {
        if let Some(pat) = pat.pat() {
            self.compile_pat(pat);
        }
    }

    fn compile_pat_or(&mut self, _pat: PatOr) {
        todo!()
    }

    fn compile_pat_list(&mut self, _pat: PatList) {
        todo!()
    }

    fn compile_pat_int(&mut self, pat: PatInt) {
        let range = pat.range();

        self.add_instr(vec![range], Instruction::PushCopy(0));

        let value = match pat.value() {
            Some(v) => v,
            None => return,
        };

        self.compile_const(range, value);
        self.add_instr(vec![range], Instruction::BinOp(BinOp::Eq));
    }

    fn compile_pat_string(&mut self, _pat: PatString) {
        todo!()
    }

    fn compile_pat_rest(&mut self, _pat: PatRest) {
        todo!()
    }

    fn compile_pat_hole(&mut self, pat: PatHole) {
        self.compile_const(pat.range(), true);
    }

    fn compile_pat_binding(&mut self, _pat: PatBinding) {
        todo!()
    }

    fn finish(&mut self) -> Func {
        if self.arity > 0 {
            self.add_instr(vec![], Instruction::PopSwap(self.arity));
        }

        self.add_instr(vec![], Instruction::Ret);

        self.debug_info.name = self.ident.as_ref().map(|v| v.name().into());

        let mut consts = self.consts.drain().collect::<Vec<_>>();
        consts.sort_by_key(|&(_, idx)| idx);
        let consts = consts.into_iter().map(|(val, _)| val).collect();

        Func {
            instructions: self.instructions.iter().copied().collect(),
            consts,
            captures: Vec::new(),
            debug_info: Some(Arc::new(std::mem::take(&mut self.debug_info))),
        }
    }
}

pub fn compile(source: Arc<Source>, expr: Expr) -> (Value, Vec<Diagnostic>) {
    match expr {
        Expr::Fn(expr) => {
            let mut compiler = Compiler::new(source);
            compiler.debug_info.range = expr.range();
            compiler.compile_expr_fn(expr);
            let func = compiler.finish();
            (func.into(), compiler.diagnostics().collect())
        }
        _ => {
            let mut compiler = Compiler::new(source);
            compiler.debug_info.range = expr.range();
            compiler.debug_info.name = Some("<thunk>".into());
            compiler.compile_expr(expr);
            let func = compiler.finish();
            (
                Thunk::new(func.into()).into(),
                compiler.diagnostics().collect(),
            )
        }
    }
}
