mod compiler;
pub mod diagnostic;
mod error;
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

pub fn compile_text(text: &str) -> (Value, Vec<Diagnostic>) {
    let source = Arc::new(Source::new("unknown.expr".into(), text.into()));
    let expr = syntax::parse(text);
    compile(source, expr)
}
