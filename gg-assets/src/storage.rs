use std::any::Any;
use std::cell::UnsafeCell;
use std::fmt::{self, Debug};

use ahash::AHashMap;
use gg_rtti::TypeId;

use crate::id::{Id, UntypedId};
use crate::Asset;

#[derive(Debug, Default)]
pub struct Storage {
    per_type: AHashMap<TypeId, Box<dyn AnyStorage>>,
}

struct TypedStorage<T> {
    entries: AHashMap<Id<T>, UnsafeCell<T>>,
}

unsafe impl<T: Sync> Sync for TypedStorage<T> {}

impl Storage {
    pub fn new() -> Storage {
        Storage::default()
    }

    fn get_storage<T: Asset>(&self) -> Option<&TypedStorage<T>> {
        let ty = TypeId::of::<T>();
        self.per_type
            .get(&ty)
            .map(|any| unsafe { any.as_any().downcast_ref().unwrap_unchecked() })
    }

    fn get_storage_mut<T: Asset>(&mut self) -> Option<&mut TypedStorage<T>> {
        let ty = TypeId::of::<T>();
        self.per_type
            .get_mut(&ty)
            .map(|any| unsafe { any.as_any_mut().downcast_mut().unwrap_unchecked() })
    }

    fn get_or_insert_storage<T: Asset>(&mut self) -> &mut TypedStorage<T> {
        let ty = TypeId::of::<T>();
        let any = self.per_type.entry(ty).or_insert_with(|| {
            Box::new(TypedStorage::<T> {
                entries: AHashMap::new(),
            })
        });

        any.as_any_mut().downcast_mut().unwrap()
    }

    pub fn insert<T: Asset>(&mut self, id: Id<T>, asset: T) {
        let storage = self.get_or_insert_storage();
        storage.entries.insert(id, UnsafeCell::new(asset));
    }

    pub fn insert_any(&mut self, id: UntypedId, ty: TypeId, asset: Box<dyn AnyAsset>) {
        self.per_type
            .entry(ty)
            .or_insert_with(|| asset.new_storage())
            .insert(id, asset);
    }

    pub fn contains_untyped(&self, id: UntypedId, ty: TypeId) -> bool {
        if let Some(storage) = self.per_type.get(&ty) {
            storage.contains(id)
        } else {
            false
        }
    }

    pub fn contains<T: Asset>(&self, id: Id<T>) -> bool {
        if let Some(storage) = self.get_storage() {
            storage.entries.contains_key(&id)
        } else {
            false
        }
    }

    pub fn get<T: Asset>(&self, id: Id<T>) -> Option<&T> {
        let storage = self.get_storage()?;
        storage.entries.get(&id).map(|v| unsafe { &*v.get() })
    }

    pub fn get_mut<T: Asset>(&mut self, id: Id<T>) -> Option<&mut T> {
        let storage = self.get_storage_mut()?;
        storage.entries.get_mut(&id).map(|v| v.get_mut())
    }

    pub unsafe fn get_mut_unsafe<T: Asset>(&self, id: Id<T>) -> Option<&mut T> {
        let storage = self.get_storage()?;
        storage.entries.get(&id).map(|v| &mut *v.get())
    }

    pub fn remove(&mut self, id: UntypedId, ty: TypeId) {
        if let Some(storage) = self.per_type.get_mut(&ty) {
            storage.remove(id);
        }
    }
}

impl<T: 'static> Debug for TypedStorage<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!("TypedStorage<{}>", std::any::type_name::<T>());
        f.debug_struct(&name)
            .field("len", &self.entries.len())
            .finish_non_exhaustive()
    }
}

pub trait AnyStorage: Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn remove(&mut self, id: UntypedId);

    fn contains(&self, id: UntypedId) -> bool;

    fn insert(&mut self, id: UntypedId, asset: Box<dyn AnyAsset>);
}

impl<T: Asset> AnyStorage for TypedStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove(&mut self, id: UntypedId) {
        self.entries.remove(&Id::from_untyped(id));
    }

    fn contains(&self, id: UntypedId) -> bool {
        self.entries.contains_key(&Id::from_untyped(id))
    }

    fn insert(&mut self, id: UntypedId, asset: Box<dyn AnyAsset>) {
        let v = asset.into_any();
        let type_got = TypeId::from(v.type_id());
        if let Ok(typed) = v.downcast::<T>() {
            self.entries.insert(Id::from_untyped(id), UnsafeCell::new(*typed));
        } else {
            tracing::error!(
                type_got = ?type_got,
                type_expected = ?TypeId::of::<T>(),
                "mismatched types"
            );
        }
    }
}

pub trait AnyAsset: Send + Sync + 'static {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn new_storage(&self) -> Box<dyn AnyStorage>;
}

impl<T: Asset> AnyAsset for T {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn new_storage(&self) -> Box<dyn AnyStorage> {
        Box::new(TypedStorage::<T> {
            entries: AHashMap::new(),
        })
    }
}
