mod compiler;
pub mod diagnostic;
mod error;
pub mod new_parser;
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
use crate::syntax::Parser;

pub fn compile_text(text: &str) -> (Value, Vec<Diagnostic>) {
    let source = Arc::new(Source::new("unknown.expr".into(), text.into()));
    let mut parser = Parser::new(source.clone());
    let expr = parser.expr();
    compile(source, &expr)
}
