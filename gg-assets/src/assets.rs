use std::ops::{Index, IndexMut};
use std::path::Path;
use std::sync::Arc;

use gg_rtti::TypeId;
use parking_lot::RwLock;

use crate::command::{new_command_channel, CommandReceiver};
use crate::handle_allocator::HandleAllocator;
use crate::loader::AssetLoaderObject;
use crate::loaders::AssetLoaders;
use crate::metadata::MetadataStorage;
use crate::shared::SharedData;
use crate::storage::Storage;
use crate::task::{new_task_channel, spawn_workers};
use crate::{Asset, AssetLoader, Handle, Id, Input, Source};

#[derive(Debug)]
pub struct Assets {
    pub(crate) storage: Storage,
    pub(crate) shared: Arc<SharedData>,
    command_receiver: CommandReceiver,
}

impl Assets {
    pub fn new<S: Source>(source: S) -> Assets {
        Self::new_from_dyn(Box::new(source))
    }

    fn new_from_dyn(source: Box<dyn Source>) -> Assets {
        let storage = Storage::new();

        let (command_sender, command_receiver) = new_command_channel();
        let (task_sender, task_receiver) = new_task_channel();

        let handle_allocator = HandleAllocator::new(command_sender.clone());
        let shared = Arc::new(SharedData {
            command_sender,
            handle_allocator,
            task_sender,
            source,
            metadata: RwLock::new(MetadataStorage::new()),
            loaders: RwLock::new(AssetLoaders::new()),
        });

        spawn_workers(shared.clone(), task_receiver);
        spawn_watcher(&shared);

        Assets {
            storage,
            shared,
            command_receiver,
        }
    }

    pub fn insert<A: Asset>(&mut self, asset: A) -> Handle<A> {
        gg_rtti::register::<A>();
        let handle = self.shared.handle_allocator.alloc();
        self.storage.insert(handle.id(), asset);
        handle
    }

    pub fn insert_defer<A: Asset>(&self, asset: A) -> Handle<A> {
        gg_rtti::register::<A>();
        let handle = self.shared.handle_allocator.alloc();
        self.shared.command_sender.insert(handle.id(), asset);
        handle
    }

    pub async fn wait_available<A: Asset>(&self, handle: &Handle<A>) {
        self.shared.wait_available(handle).await;
    }

    pub fn set_path<A, P>(&self, handle: &Handle<A>, path: P)
    where
        A: Asset,
        P: AsRef<Path>,
    {
        self.shared
            .set_path(handle.as_untyped(), path.as_ref().into());
    }

    pub fn find_by_path<A, P>(&self, path: P) -> Option<Handle<A>>
    where
        A: Asset,
        P: AsRef<Path>,
    {
        let metadata = self.shared.metadata.read();
        metadata
            .find_handle_by_path(path.as_ref(), TypeId::of::<A>())
            .map(Handle::from_untyped)
    }

    pub fn contains<A: Asset>(&self, handle: &Handle<A>) -> bool {
        self.storage.contains(handle.id())
    }

    pub fn contains_id<A: Asset>(&self, id: Id<A>) -> bool {
        self.storage.contains(id)
    }

    pub fn get<A: Asset>(&self, handle: &Handle<A>) -> Option<&A> {
        self.storage.get(handle.id())
    }

    pub fn get_mut<A: Asset>(&mut self, handle: &Handle<A>) -> Option<&mut A> {
        self.storage.get_mut(handle.id())
    }

    pub fn get_by_id<A: Asset>(&self, id: Id<A>) -> Option<&A> {
        self.storage.get(id)
    }

    pub fn get_by_id_mut<A: Asset>(&mut self, id: Id<A>) -> Option<&mut A> {
        self.storage.get_mut(id)
    }

    pub fn add_loader<A, L>(&self, loader: L)
    where
        A: Asset,
        L: AssetLoader<A>,
    {
        self.shared.add_loader(AssetLoaderObject::new(loader));
    }

    pub fn load<A, P>(&self, path: P) -> Handle<A>
    where
        A: Asset,
        P: AsRef<Path>,
    {
        self.shared.load(path)
    }

    pub fn fabricate<A, I>(&self, input: I) -> Handle<A>
    where
        A: Asset,
        I: Input,
    {
        self.shared.fabricate(input)
    }

    pub fn fabricate_with<A, I, L>(&self, input: I, loader: L) -> Handle<A>
    where
        A: Asset,
        I: Input,
        L: AssetLoader<A, Input = I>,
    {
        self.shared.fabricate_with(input, loader)
    }

    pub fn maintain(&mut self) {
        while let Some(command) = self.command_receiver.try_recv() {
            command.execute(self);
        }
    }

    pub fn defer<F>(&self, command: F)
    where
        F: FnOnce(&mut Assets) + Send + Sync + 'static,
    {
        self.shared.command_sender.closure(command);
    }
}

impl<A: Asset> Index<&Handle<A>> for Assets {
    type Output = A;

    fn index(&self, handle: &Handle<A>) -> &A {
        match self.get(handle) {
            Some(v) => v,
            None => no_such_asset(handle.id()),
        }
    }
}

impl<A: Asset> IndexMut<&Handle<A>> for Assets {
    fn index_mut(&mut self, handle: &Handle<A>) -> &mut A {
        match self.get_mut(handle) {
            Some(v) => v,
            None => no_such_asset(handle.id()),
        }
    }
}

impl<A: Asset> Index<Id<A>> for Assets {
    type Output = A;

    fn index(&self, id: Id<A>) -> &A {
        match self.get_by_id(id) {
            Some(v) => v,
            None => no_such_asset(id),
        }
    }
}

impl<A: Asset> IndexMut<Id<A>> for Assets {
    fn index_mut(&mut self, id: Id<A>) -> &mut A {
        match self.get_by_id_mut(id) {
            Some(v) => v,
            None => no_such_asset(id),
        }
    }
}

fn spawn_watcher(shared: &Arc<SharedData>) {
    let shared_copy = shared.clone();
    shared.source.start_watching(Box::new(move |path| {
        shared_copy.hot_reload(path.into());
    }));
}

#[cold]
#[inline(never)]
fn no_such_asset<A>(id: Id<A>) -> ! {
    panic!("asset {:?} does not exit", id);
}
