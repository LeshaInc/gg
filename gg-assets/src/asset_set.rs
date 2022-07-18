use std::cell::RefCell;

use smallvec::SmallVec;

use crate::handle::UntypedHandle;
use crate::shared::SharedData;
use crate::storage::Storage;
use crate::{Asset, Assets, Handle};

#[derive(Clone, Debug, Default)]
pub struct AssetSet {
    read: SmallVec<[(UntypedHandle, usize); 1]>,
    write: SmallVec<[(UntypedHandle, usize); 1]>,
}

impl AssetSet {
    pub fn new() -> AssetSet {
        AssetSet::default()
    }

    pub fn read<A: Asset>(mut self, handle: &Handle<A>) -> AssetSet {
        self.add_read(handle);
        self
    }

    pub fn write<A: Asset>(mut self, handle: &Handle<A>) -> AssetSet {
        self.add_write(handle);
        self
    }

    pub fn add_read<A: Asset>(&mut self, handle: &Handle<A>) {
        self.add_read_untyped(handle.as_untyped());
    }

    pub fn add_write<A: Asset>(&mut self, handle: &Handle<A>) {
        self.add_write_untyped(handle.as_untyped());
    }

    pub(crate) async fn wait_available(&self, shared: &SharedData) {
        for handle in self.iter_handles() {
            shared.wait_available_untyped(handle).await;
        }
    }

    pub fn load(self, assets: &mut Assets) -> Option<AssetSetAccessor<'_>> {
        for handle in self.iter_handles() {
            if !assets.storage.contains_untyped(handle.id(), handle.ty()) {
                return None;
            }
        }

        Some(AssetSetAccessor {
            storage: &mut assets.storage,
            handles: RefCell::new(self),
        })
    }

    fn add_read_untyped(&mut self, handle: &UntypedHandle) {
        if self.has_read(handle) {
            return;
        }

        if self.has_write(handle) {
            panic!("trying to read an asset while writing");
        }

        self.read.push((handle.clone(), 0));
    }

    fn add_write_untyped(&mut self, handle: &UntypedHandle) {
        if self.has_write(handle) {
            panic!("trying to simultaneously write to an asset ");
        }

        if self.has_read(handle) {
            panic!("trying write to an asset while reading");
        }

        self.write.push((handle.clone(), 0));
    }

    pub(crate) fn iter_handles(&self) -> impl Iterator<Item = &UntypedHandle> + '_ {
        self.read.iter().chain(&self.write).map(|v| &v.0)
    }

    fn has_read(&self, handle: &UntypedHandle) -> bool {
        self.read.iter().any(|v| &v.0 == handle)
    }

    fn has_write(&self, handle: &UntypedHandle) -> bool {
        self.write.iter().any(|v| &v.0 == handle)
    }

    fn borrow_read(&mut self, handle: &UntypedHandle) {
        let mut it = self.read.iter_mut().chain(&mut self.write);
        let (_, rc) = it.find(|v| &v.0 == handle).expect("no such asset in set");
        if *rc == usize::max_value() {
            panic!("trying to read asset while writing");
        }

        *rc += 1;
    }

    fn borrow_write(&mut self, handle: &UntypedHandle) {
        let mut it = self.write.iter_mut();
        let (_, rc) = it.find(|v| &v.0 == handle).expect("no such asset in set");
        if *rc > 0 {
            panic!("trying to write asset while already borrowed");
        }

        *rc = usize::max_value();
    }
}

#[derive(Debug)]
pub struct AssetSetAccessor<'a> {
    storage: &'a mut Storage,
    handles: RefCell<AssetSet>,
}

impl AssetSetAccessor<'_> {
    pub fn get<A: Asset>(&self, handle: &Handle<A>) -> &A {
        self.handles.borrow_mut().borrow_read(handle.as_untyped());
        self.storage.get(handle.id()).unwrap()
    }

    pub fn get_mut<A: Asset>(&mut self, handle: &Handle<A>) -> &mut A {
        self.handles.borrow_mut().borrow_write(handle.as_untyped());
        unsafe { self.storage.get_mut_unsafe(handle.id()).unwrap() }
    }
}
