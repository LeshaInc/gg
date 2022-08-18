use std::fmt::{self, Debug};
use std::sync::Arc;

use arc_swap::ArcSwapOption;

use crate::syntax::BinOp;
use crate::vm::{interpret, Func};

#[derive(Clone)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
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
            Value::Bool(v) => v.fmt(f),
            Value::String(v) => v.fmt(f),
            Value::Func(v) => v.fmt(f),
            Value::Thunk(v) => v.fmt(f),
            Value::List(v) => v.fmt(f),
            Value::Map(v) => v.fmt(f),
        }
    }
}

impl Value {
    pub fn to_i64(&self) -> i64 {
        match self {
            Value::Int(v) => *v,
            _ => panic!("not an integer"),
        }
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Value::Bool(true))
    }

    pub fn bin_op(&self, other: &Value, op: BinOp) -> Value {
        match op {
            BinOp::Add => Value::Int(self.to_i64() + other.to_i64()),
            BinOp::Sub => Value::Int(self.to_i64() - other.to_i64()),
            BinOp::Mul => Value::Int(self.to_i64() * other.to_i64()),
            BinOp::Pow => Value::Int(self.to_i64().pow(other.to_i64() as u32)),
            BinOp::Lt => Value::Bool(self.to_i64() < other.to_i64()),
            BinOp::Gt => Value::Bool(self.to_i64() > other.to_i64()),
            BinOp::Eq => Value::Bool(self.to_i64() == other.to_i64()),
            _ => todo!(),
        }
    }

    pub fn force_eval(&self) {
        match self {
            Value::Thunk(v) => v.force_eval(),
            Value::List(list) => {
                for value in list.iter() {
                    value.force_eval()
                }
            }
            Value::Map(map) => {
                for value in map.values() {
                    value.force_eval();
                }
            }
            _ => {}
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

    pub fn force_eval(&self) {
        let value = self.value.load();
        if value.is_some() {
            return;
        }

        let mut stack = Vec::new();
        interpret(&self.func, &mut stack);
        let value = stack.pop().unwrap();
        self.value.store(Some(Arc::new(value)));
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
