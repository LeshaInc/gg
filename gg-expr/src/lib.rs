pub mod builtins;
pub mod compiler;
pub mod diagnostic;
mod source;
pub mod syntax;
mod value;
pub mod vm;

use diagnostic::Severity;

pub use self::compiler::{compile, Compiler};
pub use self::source::{LineColPos, LineColRange, Source, SourceText};
pub use self::value::{DebugInfo, ExtFunc, Func, FuncValue, List, Map, Type, Value};
pub use self::vm::{Error, Result, Vm, VmContext};
use crate::diagnostic::Diagnostic;

pub fn compile_text(env: Map, text: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let parse_res = syntax::parse(text);

    let mut diagnostics = parse_res.diagnostics;

    let value = parse_res.expr.map(|e| {
        let mut compile_res = compile(env, parse_res.source, e);
        diagnostics.append(&mut compile_res.diagnostics);
        compile_res.func.into()
    });

    (value, diagnostics)
}

pub fn eval(env: Map, text: &str) -> (Result<Value>, Vec<Diagnostic>) {
    let (val, diagnostics) = compile_text(env, text);
    let val = match val {
        Some(v) => v,
        None => {
            return (
                Err(Error::new(Diagnostic::new(
                    Severity::Error,
                    "compilation failed",
                ))),
                diagnostics,
            )
        }
    };

    let mut vm = Vm::new();
    match vm.eval(&val, &[]) {
        Ok(v) => (Ok(v), diagnostics),
        Err(e) => (Err(e), diagnostics),
    }
}
