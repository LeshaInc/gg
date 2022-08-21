mod func;
mod thunk;

use std::fmt::{self, Debug};
use std::hint::unreachable_unchecked;
use std::mem::ManuallyDrop;
use std::sync::Arc;

pub use self::func::{DebugInfo, Func};
pub use self::thunk::Thunk;
use crate::Error;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Type {
    Null = 0,
    Int = 1,
    Float = 2,
    Bool = 3,
    String = 4,
    Func = 5,
    Thunk = 6,
    List = 7,
    Map = 8,
}

impl Type {
    pub const VALUES: [Type; 9] = [
        Type::Null,
        Type::Int,
        Type::Float,
        Type::Bool,
        Type::String,
        Type::Func,
        Type::Thunk,
        Type::List,
        Type::Map,
    ];

    fn is_heap(&self) -> bool {
        use Type::*;
        matches!(self, String | Func | Thunk | List | Map)
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Type::Null => "null",
            Type::Int => "int",
            Type::Float => "float",
            Type::Bool => "bool",
            Type::String => "string",
            Type::Func => "func",
            Type::Thunk => "thunk",
            Type::List => "list",
            Type::Map => "map",
        })
    }
}

pub struct Value {
    ty: Type,
    payload: Payload,
}

impl Value {
    fn new_heap(ty: Type, heap: HeapValue) -> Value {
        Value {
            ty,
            payload: Payload {
                heap: ManuallyDrop::new(Arc::new(heap)),
            },
        }
    }

    unsafe fn heap_make_mut(&mut self) -> &mut HeapValue {
        Arc::make_mut(&mut self.payload.heap)
    }

    pub fn null() -> Value {
        Value {
            ty: Type::Null,
            payload: Payload { null: () },
        }
    }

    pub fn ty(&self) -> Type {
        self.ty
    }

    pub fn is_null(&self) -> bool {
        self.ty == Type::Null
    }

    pub fn as_int(&self) -> Result<i64, FromValueError> {
        self.try_into()
    }

    pub fn as_float(&self) -> Result<f64, FromValueError> {
        self.try_into()
    }

    pub fn as_bool(&self) -> Result<bool, FromValueError> {
        self.try_into()
    }

    pub fn as_string(&self) -> Result<&str, FromValueError> {
        self.try_into()
    }

    pub fn as_func(&self) -> Result<&Func, FromValueError> {
        self.try_into()
    }

    pub fn as_func_mut(&mut self) -> Result<&mut Func, FromValueError> {
        if self.ty == Type::Func {
            match unsafe { self.heap_make_mut() } {
                HeapValue::Func(v) => Ok(v),
                _ => unsafe { unreachable_unchecked() },
            }
        } else {
            Err(FromValueError {
                expected: Type::Func,
                got: self.ty,
            })
        }
    }

    pub fn as_thunk(&self) -> Result<&Thunk, FromValueError> {
        self.try_into()
    }

    pub fn as_list(&self) -> Result<&im::Vector<Value>, FromValueError> {
        self.try_into()
    }

    pub fn as_map(&self) -> Result<&im::HashMap<String, Value>, FromValueError> {
        self.try_into()
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_null() && self.as_bool() != Ok(false)
    }

    pub fn force_eval(&self) -> Result<(), Error> {
        if let Ok(thunk) = self.as_thunk() {
            thunk.force_eval()?;
        } else if let Ok(list) = self.as_list() {
            for value in list.iter() {
                value.force_eval()?;
            }
        } else if let Ok(map) = self.as_map() {
            for value in map.values() {
                value.force_eval()?;
            }
        }

        Ok(())
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        if self.ty.is_heap() {
            let heap = unsafe { &self.payload.heap };

            Value {
                ty: self.ty,
                payload: Payload {
                    heap: ManuallyDrop::new(Arc::clone(&heap)),
                },
            }
        } else {
            unsafe { std::ptr::read(self) }
        }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        if self.ty.is_heap() {
            unsafe { ManuallyDrop::drop(&mut self.payload.heap) }
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            Type::Null => f.write_str("null"),
            Type::Int => self.as_int().unwrap().fmt(f),
            Type::Float => self.as_float().unwrap().fmt(f),
            Type::Bool => self.as_bool().unwrap().fmt(f),
            Type::String => self.as_string().unwrap().fmt(f),
            Type::Func => self.as_func().unwrap().fmt(f),
            Type::Thunk => self.as_thunk().unwrap().fmt(f),
            Type::List => self.as_list().unwrap().fmt(f),
            Type::Map => self.as_map().unwrap().fmt(f),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.ty != other.ty {
            return false;
        }

        match self.ty {
            Type::Null => true,
            Type::Int => self.as_int() == other.as_int(),
            Type::Float => self.as_float() == other.as_float(),
            Type::Bool => self.as_bool() == other.as_bool(),
            Type::String => self.as_string() == other.as_string(),
            Type::Func => self.as_func() == other.as_func(),
            Type::Thunk => self.as_thunk() == other.as_thunk(),
            Type::List => self.as_list() == other.as_list(),
            Type::Map => self.as_map() == other.as_map(),
        }
    }
}

impl From<i64> for Value {
    fn from(int: i64) -> Value {
        Value {
            ty: Type::Int,
            payload: Payload { int },
        }
    }
}

impl From<f64> for Value {
    fn from(float: f64) -> Value {
        Value {
            ty: Type::Float,
            payload: Payload { float },
        }
    }
}

impl From<bool> for Value {
    fn from(bool: bool) -> Value {
        Value {
            ty: Type::Bool,
            payload: Payload { bool },
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Value {
        Value::new_heap(Type::String, HeapValue::String(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Value {
        Value::new_heap(Type::String, HeapValue::String(value.into()))
    }
}

impl From<Func> for Value {
    fn from(value: Func) -> Value {
        Value::new_heap(Type::Func, HeapValue::Func(value))
    }
}

impl From<Thunk> for Value {
    fn from(value: Thunk) -> Value {
        Value::new_heap(Type::Thunk, HeapValue::Thunk(value))
    }
}

impl From<im::Vector<Value>> for Value {
    fn from(value: im::Vector<Value>) -> Value {
        Value::new_heap(Type::List, HeapValue::List(value))
    }
}

impl From<im::HashMap<String, Value>> for Value {
    fn from(value: im::HashMap<String, Value>) -> Value {
        Value::new_heap(Type::Map, HeapValue::Map(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("expected {:?}, found {:?}", self.expected, self.got)]
pub struct FromValueError {
    pub expected: Type,
    pub got: Type,
}

impl TryFrom<&Value> for i64 {
    type Error = FromValueError;

    fn try_from(value: &Value) -> Result<i64, FromValueError> {
        if value.ty == Type::Int {
            Ok(unsafe { value.payload.int })
        } else {
            Err(FromValueError {
                expected: Type::Int,
                got: value.ty,
            })
        }
    }
}

impl TryFrom<&Value> for f64 {
    type Error = FromValueError;

    fn try_from(value: &Value) -> Result<f64, FromValueError> {
        if value.ty == Type::Float {
            Ok(unsafe { value.payload.float })
        } else {
            Err(FromValueError {
                expected: Type::Float,
                got: value.ty,
            })
        }
    }
}

impl TryFrom<&Value> for bool {
    type Error = FromValueError;

    fn try_from(value: &Value) -> Result<bool, FromValueError> {
        if value.ty == Type::Bool {
            Ok(unsafe { value.payload.bool })
        } else {
            Err(FromValueError {
                expected: Type::Bool,
                got: value.ty,
            })
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = FromValueError;

    fn try_from(value: &'a Value) -> Result<&'a str, FromValueError> {
        if value.ty == Type::String {
            let heap = unsafe { &**value.payload.heap };
            match heap {
                HeapValue::String(v) => Ok(v),
                _ => unsafe { unreachable_unchecked() },
            }
        } else {
            Err(FromValueError {
                expected: Type::String,
                got: value.ty,
            })
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a Func {
    type Error = FromValueError;

    fn try_from(value: &'a Value) -> Result<&'a Func, FromValueError> {
        if value.ty == Type::Func {
            let heap = unsafe { &**value.payload.heap };
            match heap {
                HeapValue::Func(v) => Ok(v),
                _ => unsafe { unreachable_unchecked() },
            }
        } else {
            Err(FromValueError {
                expected: Type::Func,
                got: value.ty,
            })
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a Thunk {
    type Error = FromValueError;

    fn try_from(value: &'a Value) -> Result<&'a Thunk, FromValueError> {
        if value.ty == Type::Thunk {
            let heap = unsafe { &**value.payload.heap };
            match heap {
                HeapValue::Thunk(v) => Ok(v),
                _ => unsafe { unreachable_unchecked() },
            }
        } else {
            Err(FromValueError {
                expected: Type::Thunk,
                got: value.ty,
            })
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a im::Vector<Value> {
    type Error = FromValueError;

    fn try_from(value: &'a Value) -> Result<&'a im::Vector<Value>, FromValueError> {
        if value.ty == Type::List {
            let heap = unsafe { &**value.payload.heap };
            match heap {
                HeapValue::List(v) => Ok(v),
                _ => unsafe { unreachable_unchecked() },
            }
        } else {
            Err(FromValueError {
                expected: Type::List,
                got: value.ty,
            })
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a im::HashMap<String, Value> {
    type Error = FromValueError;

    fn try_from(value: &'a Value) -> Result<&'a im::HashMap<String, Value>, FromValueError> {
        if value.ty == Type::Map {
            let heap = unsafe { &**value.payload.heap };
            match heap {
                HeapValue::Map(v) => Ok(v),
                _ => unsafe { unreachable_unchecked() },
            }
        } else {
            Err(FromValueError {
                expected: Type::Map,
                got: value.ty,
            })
        }
    }
}

union Payload {
    null: (),
    int: i64,
    float: f64,
    bool: bool,
    heap: ManuallyDrop<Arc<HeapValue>>,
}

#[derive(Clone)]
enum HeapValue {
    String(String),
    Func(Func),
    Thunk(Thunk),
    List(im::Vector<Value>),
    Map(im::HashMap<String, Value>),
}
