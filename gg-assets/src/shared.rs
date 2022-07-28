use std::path::Path;
use std::sync::Arc;

use gg_util::ahash::AHashSet;
use gg_util::parking_lot::{Mutex, RwLock};
use gg_util::rtti::TypeId;
use tracing::trace;

use crate::command::CommandSender;
use crate::event::{EventKind, EventSenders};
use crate::handle::UntypedHandle;
use crate::handle_allocator::HandleAllocator;
use crate::id::UntypedId;
use crate::loader::AssetLoaderObject;
use crate::loaders::AssetLoaders;
use crate::metadata::MetadataStorage;
use crate::sync_any::SyncAny;
use crate::task::TaskSender;
use crate::{Asset, AssetLoader, Handle, Input, Source};

#[derive(Debug)]
pub struct SharedData {
    pub command_sender: CommandSender,
    pub handle_allocator: HandleAllocator,
    pub task_sender: TaskSender,
    pub source: Box<dyn Source>,
    pub metadata: RwLock<MetadataStorage>,
    pub loaders: RwLock<AssetLoaders>,
    pub event_senders: RwLock<EventSenders>,
    pub initialized_assets: Mutex<AHashSet<TypeId>>,
}

impl SharedData {
    pub fn send_event(&self, kind: EventKind, id: UntypedId, ty: TypeId) {
        self.event_senders.read().send(kind, id, ty)
    }

    pub fn set_path(&self, handle: &UntypedHandle, path: Arc<Path>) {
        let mut metadata = self.metadata.write();
        metadata.set_path_for_handle(handle, path);
    }

    pub fn add_loader(&self, loader: AssetLoaderObject) {
        self.loaders.write().insert(loader);
    }

    pub async fn wait_available<A: Asset>(&self, handle: &Handle<A>) {
        self.wait_available_untyped(handle.as_untyped()).await;
    }

    pub async fn wait_available_untyped(&self, handle: &UntypedHandle) {
        let flag = {
            let meta_storage = self.metadata.read();
            let meta = meta_storage.get(handle.id()).unwrap();
            meta.available.clone()
        };
        flag.wait(true).await;
    }

    pub fn insert<A: Asset>(&self, asset: A) -> Handle<A> {
        gg_util::rtti::register::<A>();
        let handle = self.handle_allocator.alloc();
        self.command_sender.insert(handle.id(), asset);
        handle
    }

    pub fn load<A, P>(&self, path: P) -> Handle<A>
    where
        A: Asset,
        P: AsRef<Path>,
    {
        if self.initialized_assets.lock().insert(TypeId::of::<A>()) {
            gg_util::rtti::register::<A>();
            self.loaders.write().insert_asset_loaders::<A>();
        }

        let path = path.as_ref().into();
        let asset_type = TypeId::of::<A>();
        let untyped = self.load_untyped(path, asset_type);
        Handle::from_untyped(untyped)
    }

    pub fn load_untyped(&self, path: Arc<Path>, asset_type: TypeId) -> UntypedHandle {
        let mut metadata = self.metadata.write();

        if let Some(handle) = metadata.find_handle_by_path(&path, asset_type) {
            return handle;
        }

        let handle = self.handle_allocator.alloc_untyped(asset_type);
        let permit = metadata.acquire_permit(handle.id());
        metadata.set_path_for_handle(&handle, path.clone());
        self.task_sender.load(handle.clone(), permit, path);
        handle
    }

    pub fn fabricate<A, I>(&self, input: I) -> Handle<A>
    where
        A: Asset,
        I: Input,
    {
        gg_util::rtti::register::<I>();

        if self.initialized_assets.lock().insert(TypeId::of::<A>()) {
            gg_util::rtti::register::<A>();
            self.loaders.write().insert_asset_loaders::<A>();
        }

        let input = Box::new(input);
        let asset_type = TypeId::of::<A>();
        let untyped = self.fabricate_untyped(input, asset_type);
        Handle::from_untyped(untyped)
    }

    pub fn fabricate_untyped(&self, input: Box<dyn SyncAny>, asset_type: TypeId) -> UntypedHandle {
        let handle = self.handle_allocator.alloc_untyped(asset_type);
        let permit = self.metadata.read().acquire_permit(handle.id());
        self.task_sender.fabricate(handle.clone(), permit, input);
        handle
    }

    pub fn fabricate_with<A, I, L>(&self, input: I, loader: L) -> Handle<A>
    where
        A: Asset,
        I: Input,
        L: AssetLoader<A, Input = I>,
    {
        gg_util::rtti::register::<A>();
        gg_util::rtti::register::<I>();

        let input = Box::new(input);
        let loader = AssetLoaderObject::new(loader);
        let asset_type = TypeId::of::<A>();
        let untyped = self.fabricate_with_untyped(input, loader, asset_type);
        Handle::from_untyped(untyped)
    }

    pub fn fabricate_with_untyped(
        &self,
        input: Box<dyn SyncAny>,
        loader: AssetLoaderObject,
        asset_type: TypeId,
    ) -> UntypedHandle {
        let loader_type = loader.ty();
        self.add_loader(loader);

        let handle = self.handle_allocator.alloc_untyped(asset_type);
        let permit = self.metadata.read().acquire_permit(handle.id());
        self.task_sender
            .fabricate_with(handle.clone(), permit, input, loader_type);
        handle
    }

    pub fn hot_reload(&self, path: Arc<Path>) {
        trace!(path = %path.display(), "hot reload request");
        let metadata = self.metadata.read();
        for handle in metadata.find_handles_by_path(&path) {
            trace!(asset_type = ?handle.ty(), path = %path.display(), "hot reloading");
            let permit = metadata.acquire_permit(handle.id());
            self.task_sender.reload(handle, permit);
        }
    }
}
