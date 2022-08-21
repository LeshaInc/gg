use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::Arc;

use crate::diagnostic::{Component, Diagnostic, Label, Severity, SourceComponent};
use crate::syntax::{
    BinOpExpr, CallExpr, Expr, FuncExpr, IfElseExpr, LetInExpr, Span, Spanned, UnOpExpr,
};
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
    diagnostics: Vec<Diagnostic>,
    num_captures: u32,
    arity: u32,
    stack_len: u32,
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
                .map(|(i, v)| (&**v, VarLocation::Stack(i as u32)))
                .collect(),
        }
    }
}

#[derive(Clone, Copy)]
enum VarLocation {
    Stack(u32),
    Capture(u32),
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
            diagnostics: Vec::new(),
            num_captures: 0,
            arity: args.len() as u32,
            stack_len: args.len() as u32,
        }
    }

    pub fn diagnostics(&mut self) -> impl Iterator<Item = Diagnostic> + '_ {
        self.diagnostics.drain(..)
    }

    fn add_error(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
        self.instructions.push(Instruction::Panic);
    }

    fn add_simple_error(&mut self, span: Span, message: &str, label: &str) {
        self.add_error(Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            components: vec![Component::Source(SourceComponent {
                source: self.debug_info.source.clone(),
                labels: vec![Label {
                    severity: Severity::Error,
                    span,
                    message: label.into(),
                }],
            })],
        });
    }

    fn try_from_usize<U: Default + TryFrom<usize>>(
        &mut self,
        span: Span,
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

                self.add_simple_error(span, message, &label);
                U::default()
            }
        }
    }

    fn add_instr(&mut self, spans: Vec<Span>, instr: Instruction) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(instr);
        self.debug_info.instruction_spans.push(spans);
        idx
    }

    fn compile_const(&mut self, span: Span, value: Value) {
        let idx = self.try_from_usize(span, "too many constants", self.consts.len());
        self.consts.push(value);
        self.add_instr(vec![span], Instruction::PushConst(idx));
    }

    fn compile_root(&mut self, expr: &'expr Spanned<Expr>) {
        self.debug_info.span = expr.span;
        self.compile(expr);
    }

    fn compile(&mut self, expr: &'expr Spanned<Expr>) {
        let span = expr.span;
        match &expr.item {
            Expr::Int(int) => self.compile_int(span, *int),
            Expr::Float(float) => self.compile_float(span, *float),
            Expr::String(string) => self.compile_string(span, string),
            Expr::Var(name) => self.compile_var(span, name),
            Expr::BinOp(bin_op) => self.compile_bin_op(span, bin_op),
            Expr::UnOp(un_op) => self.compile_un_op(span, un_op),
            Expr::Paren(expr) => self.compile(&expr),
            Expr::List(list) => self.compile_list(span, &list.exprs),
            Expr::Func(func) => self.compile_func(span, func),
            Expr::Call(call) => self.compile_call(span, call),
            Expr::IfElse(if_else) => self.compile_if_else(span, if_else),
            Expr::LetIn(let_in) => self.compile_let_in(span, let_in),
            Expr::Error => {}
        }

        if self.stack_len == u32::MAX {
            self.add_simple_error(span, "stack overflow", "reached `2**32-1` stack entries");
        } else {
            self.stack_len += 1;
        }
    }

    fn compile_int(&mut self, span: Span, int: i64) {
        self.compile_const(span, int.into());
    }

    fn compile_float(&mut self, span: Span, float: f64) {
        self.compile_const(span, float.into());
    }

    fn compile_string(&mut self, span: Span, string: &str) {
        self.compile_const(span, string.into());
    }

    fn compile_list(&mut self, span: Span, list: &'expr [Spanned<Expr>]) {
        for expr in list {
            self.compile(expr);
        }

        let len = self.try_from_usize(span, "list too long", list.len());
        self.add_instr(vec![span], Instruction::NewList(len));

        self.stack_len -= len;
    }

    fn compile_func(&mut self, span: Span, func: &'expr FuncExpr) {
        let mut parent = std::mem::take(self);
        let mut compiler = Compiler::new(parent.debug_info.source.clone(), &func.args);
        compiler.name = parent.inner_name;
        compiler.debug_info.span = span;
        compiler.diagnostics = std::mem::take(&mut parent.diagnostics);
        compiler.parent = Some(Box::new(parent));
        compiler.compile(&func.expr);
        let func = compiler.finish();
        *self = *compiler.parent.unwrap();
        self.diagnostics = compiler.diagnostics;

        self.compile_const(span, func.into());

        let num_captures = compiler.num_captures;
        if num_captures > 0 {
            self.add_instr(vec![span], Instruction::NewFunc(num_captures));
        }
    }

    fn compile_call(&mut self, span: Span, call: &'expr CallExpr) {
        let num_args: u32 = self.try_from_usize(span, "too many arguments", call.args.len());

        for arg in &call.args {
            self.compile(arg);
        }

        self.compile(&call.func);
        self.add_instr(vec![span, call.func.span], Instruction::Call);

        self.stack_len -= num_args;
        self.stack_len -= 1;
    }

    fn compile_var(&mut self, span: Span, name: &'expr str) {
        let mut depth = 0;
        let mut current = &mut *self;

        loop {
            if let Some(location) = current.scope.vars.get(name) {
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
                    current.scope.vars.insert(name, location);
                    current.num_captures += 1;
                    current = current.parent.as_mut().unwrap();
                }

                break;
            } else if current.name == Some(name) {
                self.add_instr(vec![], Instruction::PushFunc(depth));
                break;
            } else if let Some(parent) = &mut current.parent {
                current = parent;
                depth += 1;
            } else {
                self.no_such_binding(span, name);
                break;
            }
        }
    }

    fn bindings_in_scope(&self) -> Vec<&'expr str> {
        let mut bindings = HashSet::new();
        let mut current = &*self;
        loop {
            for binding in current.scope.vars.keys().copied().chain(current.name) {
                if !bindings.contains(binding) {
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

    fn no_such_binding(&mut self, span: Span, name: &str) {
        let mut in_scope = self.bindings_in_scope();
        in_scope.sort_by_cached_key(|v| strsim::damerau_levenshtein(v, name));

        let mut help = String::from("perhaps you meant ");
        for (i, var) in in_scope.iter().take(3).enumerate() {
            if i > 0 {
                help.push_str(", ");
            }

            let _ = write!(&mut help, "`{}`", var);
        }

        let mut diagnostic =
            Diagnostic::new(Severity::Error, format!("cannot find binding `{}`", name))
                .with_source(
                    SourceComponent::new(self.debug_info.source.clone()).with_label(
                        Severity::Error,
                        span,
                        "no such binding",
                    ),
                );

        if !in_scope.is_empty() {
            diagnostic = diagnostic.with_help(help);
        }

        self.add_error(diagnostic);
    }

    fn compile_bin_op(&mut self, span: Span, bin_op: &'expr BinOpExpr) {
        self.compile(&bin_op.lhs);
        self.compile(&bin_op.rhs);
        let spans = vec![span, bin_op.lhs.span, bin_op.rhs.span];
        self.add_instr(spans, Instruction::BinOp(bin_op.op));
        self.stack_len -= 2;
    }

    fn compile_un_op(&mut self, span: Span, un_op: &'expr UnOpExpr) {
        self.compile(&un_op.expr);
        let spans = vec![span, un_op.expr.span];
        self.add_instr(spans, Instruction::UnOp(un_op.op));
        self.stack_len -= 1;
    }

    fn compile_if_else(&mut self, span: Span, if_else: &'expr IfElseExpr) {
        self.compile(&if_else.cond);

        let start = self.add_instr(vec![span, if_else.cond.span], Instruction::Nop);
        self.stack_len -= 1;
        self.compile(&if_else.if_false);
        self.stack_len -= 1;
        let mid = self.add_instr(vec![], Instruction::Nop);
        self.compile(&if_else.if_true);
        let end = self.instructions.len();

        let offset = self.try_from_usize(span, "if expression too long", mid - start);
        self.instructions[start] = Instruction::JumpIf(offset);

        let offset = self.try_from_usize(span, "if expression too long", end - mid - 1);
        self.instructions[mid] = Instruction::Jump(offset);
    }

    fn compile_let_in(&mut self, span: Span, let_in: &'expr LetInExpr) {
        self.parent_scopes.push(self.scope.clone());

        for (binding, expr) in &let_in.vars {
            let idx = self.stack_len;
            self.inner_name = Some(&*binding);
            self.compile(expr);
            self.inner_name = None;
            self.scope.vars.insert(&*binding, VarLocation::Stack(idx));
        }

        self.compile(&let_in.expr);

        let num_vars = self.try_from_usize(span, "too many variables", let_in.vars.len());
        self.add_instr(vec![], Instruction::PopSwap(num_vars));
        self.stack_len -= num_vars;

        self.parent_scopes.pop();
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

pub fn compile(source: Arc<Source>, expr: &Spanned<Expr>) -> (Value, Vec<Diagnostic>) {
    match &expr.item {
        Expr::Func(func) => {
            let mut compiler = Compiler::new(source, &func.args);
            compiler.compile_root(&func.expr);
            let func = compiler.finish();
            (func.into(), compiler.diagnostics().collect())
        }
        _ => {
            let mut compiler = Compiler::new(source, &[]);
            compiler.debug_info.name = Some("<thunk>".into());
            compiler.compile_root(&expr);
            let func = compiler.finish();
            (
                Thunk::new(func.into()).into(),
                compiler.diagnostics().collect(),
            )
        }
    }
}
