pub mod compiler;
pub mod diagnostic;
mod error;
mod source;
pub mod syntax;
mod value;
pub mod vm;

pub use self::compiler::{compile, Compiler};
pub use self::error::{Error, Result};
pub use self::source::{LineColPos, LineColRange, Source, SourceText};
pub use self::value::{DebugInfo, Func, Type, Value};
pub use self::vm::Vm;
use crate::diagnostic::Diagnostic;

pub fn compile_text(text: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let parse_res = syntax::parse(text);

    let mut diagnostics = parse_res.diagnostics;

    let value = parse_res.expr.map(|e| {
        let mut compile_res = compile(parse_res.source, e);
        diagnostics.append(&mut compile_res.diagnostics);
        compile_res.func.into()
    });

    (value, diagnostics)
}
