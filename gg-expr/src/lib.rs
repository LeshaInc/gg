mod compiler;
pub mod diagnostic;
mod source;
pub mod syntax;
mod value;
mod vm;

use std::sync::Arc;

use syntax::Parser;

pub use self::compiler::{compile, Compiler};
pub use self::source::{Line, Source};
pub use self::value::{DebugInfo, Func, Thunk, Type, Value};
pub use self::vm::{Instruction, Vm};

pub fn compile_text(text: &str) -> Value {
    let source = Arc::new(Source::new("unknown.expr".into(), text.into()));
    let mut parser = Parser::new(source.clone());
    let expr = parser.expr();
    compile(source, &expr)
}
