mod compiler;
pub mod diagnostic;
mod error;
pub mod new_compiler;
mod source;
pub mod syntax;
mod value;
mod vm;

use std::sync::Arc;

pub use self::compiler::{compile, Compiler};
pub use self::error::Error;
pub use self::source::{Line, Source};
pub use self::value::{DebugInfo, Func, Thunk, Type, Value};
pub use self::vm::{Instruction, Vm};
use crate::diagnostic::Diagnostic;

pub fn compile_text(text: &str) -> (Option<Value>, Vec<Diagnostic>) {
    let source = Arc::new(Source::new("unknown.expr".into(), text.into()));
    let res = syntax::parse(text);

    println!("{:#?}", res.node);

    let mut diagnostics = res.errors;

    let value = res.expr.map(|e| {
        let (v, mut d) = compile(source, e);
        diagnostics.append(&mut d);
        v
    });

    (value, diagnostics)
}
