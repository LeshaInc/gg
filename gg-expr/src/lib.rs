pub mod compiler;
pub mod diagnostic;
mod error;
mod source;
pub mod syntax;
mod value;
pub mod vm;

use std::sync::Arc;

pub use self::compiler::{compile, Compiler};
pub use self::error::Error;
pub use self::source::{Line, Source};
pub use self::value::{DebugInfo, Func, Thunk, Type, Value};
use crate::diagnostic::Diagnostic;

pub fn compile_text(text: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let source = Arc::new(Source::new("unknown.expr".into(), text.into()));
    let parse_res = syntax::parse(text);

    println!("{:#?}", parse_res.node);

    let mut diagnostics = parse_res.diagnostics;

    let value = parse_res.expr.map(|e| {
        let mut compile_res = compile(source, e);
        diagnostics.append(&mut compile_res.diagnostics);
        compile_res.value
    });

    (value, diagnostics)
}
