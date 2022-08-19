use std::fmt::{self, Debug};
use std::sync::Arc;

use arc_swap::ArcSwapOption;

use crate::syntax::BinOp;
use crate::vm::{Func, Vm};

pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    Heap(Arc<HeapValue>),
}

#[derive(Clone)]
pub enum HeapValue {
    String(String),
    Func(Func),
    Thunk(Thunk),
    List(im::Vector<Value>),
    Map(im::HashMap<String, Value>),
}

impl Clone for Value {
    fn clone(&self) -> Value {
        if let Value::Heap(v) = self {
            Value::Heap(v.clone())
        } else {
            unsafe { std::ptr::read(self) }
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Int(v) => v.fmt(f),
            Value::Float(v) => v.fmt(f),
            Value::Bool(v) => v.fmt(f),
            Value::Heap(v) => match &**v {
                HeapValue::String(v) => v.fmt(f),
                HeapValue::Func(v) => v.fmt(f),
                HeapValue::Thunk(v) => v.fmt(f),
                HeapValue::List(v) => v.fmt(f),
                HeapValue::Map(v) => v.fmt(f),
            },
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

    pub fn as_func(&self) -> Option<&Func> {
        match self {
            Value::Heap(v) => match &**v {
                HeapValue::Func(func) => Some(func),
                _ => None,
            },
            _ => None,
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
        let value = match self {
            Value::Heap(v) => &**v,
            _ => return,
        };

        match value {
            HeapValue::Thunk(v) => v.force_eval(),
            HeapValue::List(list) => {
                for value in list.iter() {
                    value.force_eval()
                }
            }
            HeapValue::Map(map) => {
                for value in map.values() {
                    value.force_eval();
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
pub struct Thunk {
    pub func: Value,
    pub value: Arc<ArcSwapOption<Value>>,
}

impl Thunk {
    pub fn new(func: Value) -> Thunk {
        Thunk {
            func,
            value: Arc::new(ArcSwapOption::new(None)),
        }
    }

    pub fn force_eval(&self) {
        let value = self.value.load();
        if value.is_some() {
            return;
        }

        let mut vm = Vm::new();
        let res = vm.eval(self.func.clone());
        self.value.store(Some(Arc::new(res)));
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
