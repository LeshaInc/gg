use std::fmt::{self, Debug};
use std::sync::Arc;

use arc_swap::ArcSwapOption;

use crate::vm::Func;

#[derive(Clone)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    String(Arc<String>),
    Func(Arc<Func>),
    Thunk(Arc<Thunk>),
    List(Box<im::Vector<Value>>),
    Map(Box<im::HashMap<Arc<String>, Value>>),
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Int(v) => v.fmt(f),
            Value::Float(v) => v.fmt(f),
            Value::String(v) => v.fmt(f),
            Value::Func(v) => v.fmt(f),
            Value::Thunk(v) => v.fmt(f),
            Value::List(v) => v.fmt(f),
            Value::Map(v) => v.fmt(f),
        }
    }
}

pub struct Thunk {
    pub func: Func,
    pub value: ArcSwapOption<Value>,
}

impl Thunk {
    pub fn new(func: Func) -> Thunk {
        Thunk {
            func,
            value: ArcSwapOption::new(None),
        }
    }
}

impl Debug for Thunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opt = self.value.load();
        if let Some(val) = &*opt {
            val.fmt(f)
        } else {
            writeln!(f, "thunk: {:?}", self.func)
        }
    }
}
