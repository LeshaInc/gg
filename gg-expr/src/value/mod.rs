mod ext_func;
mod func;

use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::hint::unreachable_unchecked;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Acquire, Release};

pub use self::ext_func::ExtFunc;
pub use self::func::{DebugInfo, Func};

pub type List = im::Vector<Value>;
pub type Map = im::HashMap<Value, Value>;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Type {
    Null = 0,
    Int = 1,
    Float = 2,
    Bool = 3,
    String = 4,
    Func = 5,
    ExtFunc = 6,
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
        Type::ExtFunc,
        Type::List,
        Type::Map,
    ];

    fn is_heap(&self) -> bool {
        use Type::*;
        matches!(self, String | Func | ExtFunc | List | Map)
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
            Type::ExtFunc => "ext_func",
            Type::List => "list",
            Type::Map => "map",
        })
    }
}

const TAG_MASK: u64 = 15;

#[repr(C)]
#[cfg(target_pointer_width = "64")]
pub union Value {
    u64: u64,
    ptr: *mut HeapValue,
}

#[repr(align(16))]
struct HeapValue {
    refcount: AtomicUsize,
    payload: HeapPayload,
}

union HeapPayload {
    string: ManuallyDrop<String>,
    func: ManuallyDrop<Func>,
    ext_func: ManuallyDrop<ExtFunc>,
    list: ManuallyDrop<List>,
    map: ManuallyDrop<Map>,
}

impl Value {
    pub fn null() -> Value {
        Value { u64: 0 }
    }

    pub fn is_null(&self) -> bool {
        self.ty() == Type::Null
    }

    pub fn ty(&self) -> Type {
        match unsafe { self.u64 & TAG_MASK } {
            0 => Type::Null,
            1 => Type::Int,
            2 => Type::Float,
            3 => Type::Bool,
            4 => Type::String,
            5 => Type::Func,
            6 => Type::ExtFunc,
            7 => Type::List,
            8 => Type::Map,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    pub fn from_int(v: i32) -> Value {
        Value {
            u64: (v as u64) << 32 | (Type::Int as u64),
        }
    }

    pub fn is_int(&self) -> bool {
        self.ty() == Type::Int
    }

    pub fn as_int(&self) -> Result<i32, FromValueError> {
        if self.is_int() {
            unsafe { Ok((self.u64 >> 32) as i32) }
        } else {
            Err(FromValueError {
                expected: Type::Int,
                got: self.ty(),
            })
        }
    }

    pub fn from_float(v: f32) -> Value {
        Value {
            u64: u64::from(v.to_bits()) << 32 | (Type::Float as u64),
        }
    }

    pub fn is_float(&self) -> bool {
        self.ty() == Type::Float
    }

    pub fn as_float(&self) -> Result<f32, FromValueError> {
        if self.is_float() {
            unsafe { Ok(f32::from_bits((self.u64 >> 32) as u32)) }
        } else {
            Err(FromValueError {
                expected: Type::Float,
                got: self.ty(),
            })
        }
    }

    pub fn from_bool(v: bool) -> Value {
        Value {
            u64: (v as u64) << 32 | (Type::Bool as u64),
        }
    }

    pub fn is_bool(&self) -> bool {
        self.ty() == Type::Bool
    }

    pub fn as_bool(&self) -> Result<bool, FromValueError> {
        if self.is_bool() {
            unsafe { Ok((self.u64 >> 32) != 0) }
        } else {
            Err(FromValueError {
                expected: Type::Bool,
                got: self.ty(),
            })
        }
    }

    pub fn is_truthy(&self) -> bool {
        !self.is_null() && self.as_bool() != Ok(false)
    }

    fn from_heap(ty: Type, heap: HeapValue) -> Value {
        let mut v = Value {
            ptr: Box::into_raw(Box::new(heap)),
        };
        unsafe {
            v.u64 |= ty as u64;
        }
        v
    }

    fn is_heap(&self) -> bool {
        self.ty().is_heap()
    }

    unsafe fn get_heap(&self) -> &HeapValue {
        let mut v = ManuallyDrop::new(std::ptr::read(self));
        v.u64 &= !TAG_MASK;
        &*v.ptr
    }

    unsafe fn get_heap_mut(&mut self) -> &mut HeapValue {
        let mut v = ManuallyDrop::new(std::ptr::read(self));
        v.u64 &= !TAG_MASK;
        &mut *v.ptr
    }

    pub fn from_string(string: String) -> Value {
        Value::from_heap(
            Type::String,
            HeapValue {
                refcount: AtomicUsize::new(1),
                payload: HeapPayload {
                    string: ManuallyDrop::new(string),
                },
            },
        )
    }

    pub fn is_string(&self) -> bool {
        self.ty() == Type::String
    }

    pub fn as_string(&self) -> Result<&str, FromValueError> {
        if self.is_string() {
            unsafe { Ok(&self.get_heap().payload.string) }
        } else {
            Err(FromValueError {
                expected: Type::String,
                got: self.ty(),
            })
        }
    }

    pub fn from_func(func: Func) -> Value {
        Value::from_heap(
            Type::Func,
            HeapValue {
                refcount: AtomicUsize::new(1),
                payload: HeapPayload {
                    func: ManuallyDrop::new(func),
                },
            },
        )
    }

    pub fn is_func(&self) -> bool {
        self.ty() == Type::Func
    }

    pub fn as_func(&self) -> Result<&Func, FromValueError> {
        if self.is_func() {
            unsafe { Ok(&self.get_heap().payload.func) }
        } else {
            Err(FromValueError {
                expected: Type::Func,
                got: self.ty(),
            })
        }
    }

    pub fn from_ext_func(ext_func: ExtFunc) -> Value {
        Value::from_heap(
            Type::ExtFunc,
            HeapValue {
                refcount: AtomicUsize::new(1),
                payload: HeapPayload {
                    ext_func: ManuallyDrop::new(ext_func),
                },
            },
        )
    }

    pub fn is_ext_func(&self) -> bool {
        self.ty() == Type::ExtFunc
    }

    pub fn as_ext_func(&self) -> Result<&ExtFunc, FromValueError> {
        if self.is_ext_func() {
            unsafe { Ok(&self.get_heap().payload.ext_func) }
        } else {
            Err(FromValueError {
                expected: Type::Func,
                got: self.ty(),
            })
        }
    }

    pub fn from_list(list: List) -> Value {
        Value::from_heap(
            Type::List,
            HeapValue {
                refcount: AtomicUsize::new(1),
                payload: HeapPayload {
                    list: ManuallyDrop::new(list),
                },
            },
        )
    }

    pub fn is_list(&self) -> bool {
        self.ty() == Type::List
    }

    pub fn as_list(&self) -> Result<&List, FromValueError> {
        if self.is_list() {
            unsafe { Ok(&self.get_heap().payload.list) }
        } else {
            Err(FromValueError {
                expected: Type::List,
                got: self.ty(),
            })
        }
    }

    pub fn from_map(map: Map) -> Value {
        Value::from_heap(
            Type::Map,
            HeapValue {
                refcount: AtomicUsize::new(1),
                payload: HeapPayload {
                    map: ManuallyDrop::new(map),
                },
            },
        )
    }

    pub fn is_map(&self) -> bool {
        self.ty() == Type::Map
    }

    pub fn as_map(&self) -> Result<&Map, FromValueError> {
        if self.is_map() {
            unsafe { Ok(&self.get_heap().payload.map) }
        } else {
            Err(FromValueError {
                expected: Type::Map,
                got: self.ty(),
            })
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        unsafe {
            if self.is_heap() {
                self.get_heap().refcount.fetch_add(1, Acquire);
            }

            std::ptr::read(self)
        }
    }
}

impl Drop for Value {
    fn drop(&mut self) {
        if !self.is_heap() {
            return;
        }

        unsafe {
            if self.get_heap().refcount.fetch_sub(1, Release) == 1 {
                drop_slow(self)
            }
        }
    }
}

#[inline(never)]
unsafe fn drop_slow(value: &mut Value) {
    let ty = value.ty();
    let payload = &mut value.get_heap_mut().payload;
    match ty {
        Type::Null | Type::Int | Type::Float | Type::Bool => unreachable_unchecked(),
        Type::String => ManuallyDrop::drop(&mut payload.string),
        Type::Func => ManuallyDrop::drop(&mut payload.func),
        Type::ExtFunc => ManuallyDrop::drop(&mut payload.ext_func),
        Type::List => ManuallyDrop::drop(&mut payload.list),
        Type::Map => ManuallyDrop::drop(&mut payload.map),
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty() {
            Type::Null => f.write_str("null"),
            Type::Int => self.as_int().unwrap().fmt(f),
            Type::Float => self.as_float().unwrap().fmt(f),
            Type::Bool => self.as_bool().unwrap().fmt(f),
            Type::String => self.as_string().unwrap().fmt(f),
            Type::Func => self.as_func().unwrap().fmt(f),
            Type::ExtFunc => self.as_ext_func().unwrap().fmt(f),
            Type::List => self.as_list().unwrap().fmt(f),
            Type::Map => fmt_map(self.as_map().unwrap(), f),
        }
    }
}

fn fmt_map(map: &im::HashMap<Value, Value>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{{")?;

    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }

        // TODO: fancier string keys
        write!(f, "[{:?}] = {:?}", k, v)?;
    }

    write!(f, "}}")
}

impl Eq for Value {}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.ty() != other.ty() {
            return false;
        }

        match self.ty() {
            Type::Null => true,
            Type::Int => self.as_int() == other.as_int(),
            Type::Float => {
                let (a, b) = (self.as_float().unwrap(), other.as_float().unwrap());
                if a.is_nan() {
                    b.is_nan()
                } else {
                    a == b
                }
            }
            Type::Bool => self.as_bool() == other.as_bool(),
            Type::String => self.as_string() == other.as_string(),
            Type::Func => self.as_func() == other.as_func(),
            Type::ExtFunc => self.as_ext_func() == other.as_ext_func(),
            Type::List => self.as_list() == other.as_list(),
            Type::Map => self.as_map() == other.as_map(),
        }
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ty().hash(state);

        match self.ty() {
            Type::Null => {}
            Type::Int => {
                self.as_int().unwrap().hash(state);
            }
            Type::Float => {
                let x = self.as_float().unwrap();
                if x.is_nan() { f32::NAN } else { x }.to_bits().hash(state);
            }
            Type::Bool => {
                self.as_bool().unwrap().hash(state);
            }
            Type::String => {
                self.as_string().unwrap().hash(state);
            }
            Type::Func => {
                self.as_func().unwrap().hash(state);
            }
            Type::ExtFunc => {
                self.as_ext_func().unwrap().hash(state);
            }
            Type::List => {
                self.as_list().unwrap().hash(state);
            }
            Type::Map => {
                self.as_map().unwrap().hash(state);
            }
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Value {
        Value::from_int(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Value {
        Value::from_float(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Value {
        Value::from_bool(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Value {
        Value::from_string(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Value {
        Value::from_string(v.into())
    }
}

impl From<Func> for Value {
    fn from(v: Func) -> Value {
        Value::from_func(v)
    }
}

impl From<ExtFunc> for Value {
    fn from(v: ExtFunc) -> Value {
        Value::from_ext_func(v)
    }
}

impl From<List> for Value {
    fn from(v: List) -> Value {
        Value::from_list(v)
    }
}

impl From<Map> for Value {
    fn from(v: Map) -> Value {
        Value::from_map(v)
    }
}

impl TryFrom<&Value> for i32 {
    type Error = FromValueError;
    fn try_from(v: &Value) -> Result<i32, FromValueError> {
        v.as_int()
    }
}

impl TryFrom<&Value> for f32 {
    type Error = FromValueError;
    fn try_from(v: &Value) -> Result<f32, FromValueError> {
        v.as_float()
    }
}

impl TryFrom<&Value> for bool {
    type Error = FromValueError;
    fn try_from(v: &Value) -> Result<bool, FromValueError> {
        v.as_bool()
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = FromValueError;
    fn try_from(v: &'a Value) -> Result<&'a str, FromValueError> {
        v.as_string()
    }
}

impl<'a> TryFrom<&'a Value> for &'a Func {
    type Error = FromValueError;
    fn try_from(v: &'a Value) -> Result<&'a Func, FromValueError> {
        v.as_func()
    }
}

impl<'a> TryFrom<&'a Value> for &'a ExtFunc {
    type Error = FromValueError;
    fn try_from(v: &'a Value) -> Result<&'a ExtFunc, FromValueError> {
        v.as_ext_func()
    }
}

impl<'a> TryFrom<&'a Value> for &'a List {
    type Error = FromValueError;
    fn try_from(v: &'a Value) -> Result<&'a List, FromValueError> {
        v.as_list()
    }
}

impl<'a> TryFrom<&'a Value> for &'a Map {
    type Error = FromValueError;
    fn try_from(v: &'a Value) -> Result<&'a Map, FromValueError> {
        v.as_map()
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct FuncValue(Value);

impl Deref for FuncValue {
    type Target = Func;

    fn deref(&self) -> &Func {
        unsafe { self.0.as_func().unwrap_unchecked() }
    }
}

impl From<FuncValue> for Value {
    fn from(v: FuncValue) -> Self {
        v.0
    }
}

impl TryFrom<Value> for FuncValue {
    type Error = FromValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value.is_func() {
            Ok(FuncValue(value))
        } else {
            Err(FromValueError {
                expected: Type::Func,
                got: value.ty(),
            })
        }
    }
}

impl Debug for FuncValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("expected {:?}, found {:?}", self.expected, self.got)]
pub struct FromValueError {
    pub expected: Type,
    pub got: Type,
}
