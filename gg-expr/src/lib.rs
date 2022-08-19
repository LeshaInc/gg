mod compiler;
pub mod diagnostic;
pub mod syntax;
mod value;
mod vm;

pub use self::compiler::{compile, Compiler};
pub use self::value::{Func, Thunk, Type, Value};
pub use self::vm::{Instruction, Vm};
