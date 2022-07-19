use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use eyre::{bail, eyre, Context, Result};
use gg_rtti::TypeId;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::OwnedSemaphorePermit;
use tracing::{error, instrument, trace};

use crate::handle::UntypedHandle;
use crate::loader::AssetLoaderObject;
use crate::metadata::Dependencies;
use crate::shared::SharedData;
use crate::storage::AnyAsset;
use crate::sync_any::SyncAny;
use crate::LoaderCtx;

pub fn spawn_workers(shared: Arc<SharedData>, mut task_receiver: TaskReceiver) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("assets-{}", id)
        })
        .build()
        .expect("failed to create tokio runtime");

    thread::Builder::new()
        .name("assets".into())
        .spawn(move || {
            let rt = &runtime;
            runtime.block_on(async move {
                while let Some(task) = task_receiver.recv().await {
                    let shared = shared.clone();
                    rt.spawn(task.execute(shared));
                }
            });
        })
        .expect("failed to spawn thread");
}

pub fn new_task_channel() -> (TaskSender, TaskReceiver) {
    let (sender, receiver) = unbounded_channel();
    (TaskSender { sender }, TaskReceiver { receiver })
}

#[derive(Debug)]
pub struct TaskReceiver {
    receiver: UnboundedReceiver<Task>,
}

impl TaskReceiver {
    async fn recv(&mut self) -> Option<Task> {
        self.receiver.recv().await
    }
}

#[derive(Clone, Debug)]
pub struct TaskSender {
    sender: UnboundedSender<Task>,
}

impl TaskSender {
    fn send(&self, task: Task) {
        let _ = self.sender.send(task);
    }

    pub fn load(
        &self,
        handle: UntypedHandle,
        permit: Option<OwnedSemaphorePermit>,
        path: Arc<Path>,
    ) {
        self.send(Task {
            path: Some(path),
            ..Task::new_untyped(handle, permit)
        });
    }

    pub fn fabricate(
        &self,
        handle: UntypedHandle,
        permit: Option<OwnedSemaphorePermit>,
        input: Box<dyn SyncAny>,
    ) {
        self.send(Task {
            input: Some(input),
            ..Task::new_untyped(handle, permit)
        });
    }

    pub fn fabricate_with(
        &self,
        handle: UntypedHandle,
        permit: Option<OwnedSemaphorePermit>,
        input: Box<dyn SyncAny>,
        loader_type: TypeId,
    ) {
        self.send(Task {
            input: Some(input),
            loader_type: Some(loader_type),
            ..Task::new_untyped(handle, permit)
        });
    }

    pub fn reload(&self, handle: UntypedHandle, permit: Option<OwnedSemaphorePermit>) {
        self.send(Task {
            reload: true,
            ..Task::new_untyped(handle, permit)
        });
    }
}

struct Task {
    handle: UntypedHandle,
    path: Option<Arc<Path>>,
    input: Option<Box<dyn SyncAny>>,
    loader_type: Option<TypeId>,
    permit: Option<OwnedSemaphorePermit>,
    reload: bool,
}

impl Task {
    fn new_untyped(handle: UntypedHandle, permit: Option<OwnedSemaphorePermit>) -> Task {
        Task {
            handle,
            permit,
            path: None,
            input: None,
            loader_type: None,
            reload: false,
        }
    }

    fn get_input(&mut self) -> Box<dyn SyncAny> {
        self.path
            .take()
            .map(|path| {
                trace!(path = %path.display());
                Box::new(path) as Box<dyn SyncAny>
            })
            .or_else(|| self.input.take())
            .unwrap_or_else(|| Box::new(()) as Box<dyn SyncAny>)
    }

    fn get_loader(&self, shared: &SharedData, input: &dyn SyncAny) -> Result<TypeId> {
        let loaders = shared.loaders.read();

        let mut selected_loader = self.loader_type;
        let input_type = TypeId::from(input.type_id());

        if selected_loader.is_none() {
            let loader_list = loaders.lookup(self.handle.ty(), input_type);
            for &loader_type in loader_list {
                let loader = loaders.get(loader_type);
                if loader.filter(input) {
                    selected_loader = Some(loader_type);
                }
            }
        }

        selected_loader.ok_or_else(|| eyre!("no loader"))
    }

    #[instrument(skip_all, fields(id = ?self.handle.id()))]
    async fn execute(self, shared: Arc<SharedData>) -> Result<()> {
        if let Err(error) = self.execute_inner(shared).await {
            error!(?error);
        }

        Ok(())
    }

    async fn ensure_permit(&mut self, shared: &SharedData) {
        if self.permit.is_none() {
            let lock = {
                let metadata = shared.metadata.read();
                metadata.get(self.handle.id()).map(|meta| meta.lock.clone())
            };

            if let Some(lock) = lock {
                self.permit = lock.acquire_owned().await.ok();
            }
        }
    }

    fn prepare_load(
        &mut self,
        shared: &SharedData,
    ) -> Result<(Box<dyn SyncAny>, AssetLoaderObject)> {
        let (input, loader_type) = if self.reload {
            let mut meta_storage = shared.metadata.write();
            let meta = meta_storage.get_or_insert(&self.handle);

            if let (Some(input), Some(loader)) = (meta.loader_input.take(), meta.loader_type) {
                (input, loader)
            } else {
                bail!("no loader info");
            }
        } else {
            let input = self.get_input();
            let loader = self.get_loader(shared, &*input)?;
            (input, loader)
        };

        let loader = shared.loaders.read().get(loader_type).clone();
        Ok((input, loader))
    }

    async fn load(
        &self,
        shared: Arc<SharedData>,
        input: &dyn SyncAny,
        loader: &AssetLoaderObject,
    ) -> Result<(Dependencies, Box<dyn AnyAsset>)> {
        let mut ctx = LoaderCtx::new(shared.clone());
        let asset = loader.load(&mut ctx, input).await.wrap_err_with(|| {
            let path = (*input).as_any().downcast_ref::<Arc<Path>>();
            if let Some(path) = path {
                format!("failed to load asset from {}", path.display())
            } else {
                "failed to load asset".into()
            }
        })?;

        Ok((ctx.into_dependencies(), asset))
    }

    async fn execute_inner(mut self, shared: Arc<SharedData>) -> Result<()> {
        self.ensure_permit(&shared).await;

        let (input, loader) = self.prepare_load(&shared)?;

        trace!(
            asset_type = ?self.handle.ty(),
            loader_type = ?loader.ty(),
            input_type = ?TypeId::from(input.type_id())
        );

        let (deps, asset) = self.load(shared.clone(), &*input, &loader).await?;

        let mut meta_storage = shared.metadata.write();

        for dep in &deps.handles {
            let meta = meta_storage.get_or_insert(dep);
            meta.rev_deps.asset_ids.insert(self.handle.id());
        }

        let meta = meta_storage.get_or_insert(&self.handle);

        meta.loader_type = Some(loader.ty());
        meta.loader_input = Some(input);
        meta.deps = deps;

        let rev_deps = std::mem::take(&mut meta.rev_deps);

        shared
            .command_sender
            .insert_untyped(self.handle.id(), self.handle.ty(), asset);

        trace!("asset loaded");

        for &rev_dep in &rev_deps.asset_ids {
            if let Some(handle) = meta_storage
                .get(rev_dep)
                .and_then(|meta| meta.handle.upgrade())
            {
                trace!(id = ?handle.id(), ty = ?handle.ty(), "cascade hot reload");
                let permit = meta_storage.acquire_permit(rev_dep);
                shared.task_sender.reload(handle, permit);
            }
        }

        Ok(())
    }
}
