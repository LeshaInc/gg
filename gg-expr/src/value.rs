use crate::vm::Func;

#[derive(Clone, Debug)]
pub enum Value {
    Int(i32),
    Float(f32),
    Func(Func),
}
