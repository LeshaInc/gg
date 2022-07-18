use std::path::Path;
use std::sync::Arc;

use eyre::Result;
use tokio::sync::oneshot;

use crate::asset_set::AssetSet;
use crate::metadata::Dependencies;
use crate::shared::SharedData;
use crate::{Asset, AssetLoader, AssetSetAccessor, Handle, Input};

pub struct LoaderCtx {
    shared: Arc<SharedData>,
    dependencies: Dependencies,
}

impl LoaderCtx {
    pub(crate) fn new(shared: Arc<SharedData>) -> LoaderCtx {
        LoaderCtx {
            shared,
            dependencies: Dependencies::default(),
        }
    }

    pub(crate) fn into_dependencies(self) -> Dependencies {
        self.dependencies
    }

    pub fn read_bytes<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>> {
        self.read_bytes_inner(path.as_ref().into())
    }

    fn read_bytes_inner(&mut self, path: Arc<Path>) -> Result<Vec<u8>> {
        let data = self.shared.source.read_bytes(&path)?;
        self.dependencies.paths.push(path);
        Ok(data)
    }

    pub fn read_string<P: AsRef<Path>>(&mut self, path: P) -> Result<String> {
        self.read_string_inner(path.as_ref().into())
    }

    fn read_string_inner(&mut self, path: Arc<Path>) -> Result<String> {
        let data = self.shared.source.read_string(&path)?;
        self.dependencies.paths.push(path);
        Ok(data)
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

    pub async fn access<F, R>(&mut self, set: AssetSet, callback: F) -> R
    where
        F: FnOnce(AssetSetAccessor) -> R + Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        for handle in set.iter_handles() {
            self.dependencies.handles.push(handle.clone());
        }

        set.wait_available(&self.shared).await;

        let (tx, rx) = oneshot::channel();

        self.shared.command_sender.closure(move |assets| {
            let set = set.load(assets).unwrap();
            let result = callback(set);
            let _ = tx.send(result);
        });

        rx.await.unwrap()
    }
}
