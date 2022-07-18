use std::fmt::{self, Debug};
use std::path::Path;
use std::sync::Arc;

use ahash::{AHashMap, AHashSet};
use gg_rtti::TypeId;
use smallvec::SmallVec;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::flag::Flag;
use crate::handle::{UntypedHandle, UntypedWeakHandle};
use crate::id::UntypedId;
use crate::sync_any::SyncAny;

pub struct Metadata {
    pub handle: UntypedWeakHandle,
    pub path: Option<Arc<Path>>,
    pub available: Arc<Flag>,
    pub lock: Arc<Semaphore>,
    pub loader_type: Option<TypeId>,
    pub loader_input: Option<Box<dyn SyncAny>>,
    pub deps: Dependencies,
    pub rev_deps: RevDependencies,
}

impl Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metadata")
            .field("path", &self.path)
            .field("loader_type", &self.loader_type)
            .field("dependencies", &self.deps)
            .finish_non_exhaustive()
    }
}

impl Metadata {
    pub fn new(handle: UntypedWeakHandle) -> Metadata {
        Metadata {
            handle,
            path: None,
            available: Arc::new(Flag::new(false)),
            lock: Arc::new(Semaphore::new(1)),
            loader_type: None,
            loader_input: None,
            deps: Dependencies::default(),
            rev_deps: RevDependencies::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct MetadataStorage {
    path_to_id: AHashMap<Arc<Path>, IdList>,
    id_to_meta: AHashMap<UntypedId, Metadata>,
}

type IdList = SmallVec<[(UntypedId, TypeId); 1]>;

impl MetadataStorage {
    pub fn new() -> MetadataStorage {
        MetadataStorage::default()
    }

    pub fn get_or_insert(&mut self, handle: &UntypedHandle) -> &mut Metadata {
        let id = handle.id();
        self.id_to_meta
            .entry(id)
            .or_insert_with(|| Metadata::new(handle.downgrade()))
    }

    pub fn remove(&mut self, id: UntypedId) {
        if let Some(metadata) = self.id_to_meta.remove(&id) {
            if let Some(path) = metadata.path {
                self.remove_path(&path, id);
            }
        }
    }

    pub fn get(&self, id: UntypedId) -> Option<&Metadata> {
        self.id_to_meta.get(&id)
    }

    pub fn set_path_for_handle(&mut self, handle: &UntypedHandle, path: Arc<Path>) {
        let meta = self.get_or_insert(handle);

        if let Some(old_path) = meta.path.replace(path.clone()) {
            self.remove_path(&old_path, handle.id());
        }

        self.insert_path(path, handle.id(), handle.ty());
    }

    pub fn insert_path(&mut self, path: Arc<Path>, id: UntypedId, ty: TypeId) {
        let list = self.path_to_id.entry(path).or_default();
        list.push((id, ty));
    }

    pub fn remove_path(&mut self, path: &Path, id: UntypedId) {
        let list = match self.path_to_id.get_mut(path) {
            Some(v) => v,
            None => return,
        };

        list.retain(|v| v.0 != id);
    }

    pub fn find_id_by_path(&self, path: &Path, ty: TypeId) -> Option<UntypedId> {
        let list = self.path_to_id.get(path)?;
        list.iter().find(|v| v.1 == ty).map(|v| v.0)
    }

    pub fn find_handle_by_path(&self, path: &Path, ty: TypeId) -> Option<UntypedHandle> {
        let id = self.find_id_by_path(path, ty)?;
        let meta = self.get(id)?;
        meta.handle.upgrade()
    }

    pub fn find_handles_by_path(&self, path: &Path) -> impl Iterator<Item = UntypedHandle> + '_ {
        self.path_to_id.get(path).into_iter().flat_map(|list| {
            list.iter()
                .flat_map(|(id, _)| self.id_to_meta.get(id))
                .flat_map(|meta| meta.handle.upgrade())
        })
    }

    pub fn acquire_permit(&self, id: UntypedId) -> Option<OwnedSemaphorePermit> {
        let lock = self.get(id)?.lock.clone();
        lock.try_acquire_owned().ok()
    }
}

#[derive(Debug, Default)]
pub struct Dependencies {
    pub paths: Vec<Arc<Path>>,
    pub handles: Vec<UntypedHandle>,
}

#[derive(Debug, Default)]
pub struct RevDependencies {
    pub asset_ids: AHashSet<UntypedId>,
}
