use std::path::Path;
use std::sync::Arc;
use std::{any, fmt};

use ahash::AHashMap;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct TypeId(any::TypeId);

impl TypeId {
    pub fn type_name(self) -> Option<&'static str> {
        type_name_of_id(self)
    }

    #[inline]
    pub fn of<T: 'static>() -> TypeId {
        TypeId(any::TypeId::of::<T>())
    }
}

impl fmt::Debug for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.type_name() {
            Some(name) => write!(f, "TypeId({})", name),
            None => self.0.fmt(f),
        }
    }
}

impl From<any::TypeId> for TypeId {
    fn from(v: any::TypeId) -> TypeId {
        TypeId(v)
    }
}

impl From<TypeId> for any::TypeId {
    fn from(v: TypeId) -> any::TypeId {
        v.0
    }
}

#[derive(Debug)]
struct TypeInfo {
    type_name: &'static str,
}

impl TypeInfo {
    fn of<T: 'static>() -> TypeInfo {
        TypeInfo {
            type_name: std::any::type_name::<T>(),
        }
    }
}

#[derive(Debug, Default)]
struct Registry {
    mapping: AHashMap<TypeId, TypeInfo>,
}

impl Registry {
    fn new() -> Registry {
        let mut registry = Registry::default();
        registry.register::<Arc<Path>>();
        registry
    }

    fn register<T: 'static>(&mut self) {
        self.mapping.insert(TypeId::of::<T>(), TypeInfo::of::<T>());
    }

    fn get(&self, ty: TypeId) -> Option<&TypeInfo> {
        self.mapping.get(&ty)
    }
}

static GLOBAL_REGISTRY: Lazy<RwLock<Registry>> = Lazy::new(|| RwLock::new(Registry::new()));

pub fn register<T: 'static>() {
    let mut registry = GLOBAL_REGISTRY.write();
    registry.register::<T>();
}

pub fn type_name_of_id(ty: TypeId) -> Option<&'static str> {
    let registry = GLOBAL_REGISTRY.read();
    registry.get(ty).map(|v| v.type_name)
}
