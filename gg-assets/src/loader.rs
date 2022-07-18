use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use eyre::{bail, Result};
use gg_rtti::TypeId;
use serde::de::DeserializeOwned;

use crate::storage::AnyAsset;
use crate::sync_any::SyncAny;
use crate::{Asset, LoaderCtx};

#[async_trait]
pub trait AssetLoader<A: Asset>: Send + Sync + 'static {
    type Input: Send + Sync + 'static;

    fn filter(&self, input: &Self::Input) -> bool {
        let _ = input;
        true
    }

    async fn load(&self, ctx: &mut LoaderCtx, input: &Self::Input) -> Result<A>;
}

#[async_trait]
pub trait BytesAssetLoader<A: Asset>: Send + Sync + 'static {
    async fn load(&self, ctx: &mut LoaderCtx, bytes: Vec<u8>) -> Result<A>;
}

#[async_trait]
impl<A, L> AssetLoader<A> for L
where
    A: Asset,
    L: BytesAssetLoader<A>,
{
    type Input = Arc<Path>;

    async fn load(&self, ctx: &mut LoaderCtx, path: &Arc<Path>) -> Result<A> {
        let bytes = ctx.read_bytes(path)?;
        self.load(ctx, bytes).await
    }
}

pub struct JsonAssetLoader<A>(PhantomData<fn() -> A>);

#[async_trait]
impl<A> BytesAssetLoader<A> for JsonAssetLoader<A>
where
    A: Asset + DeserializeOwned,
{
    async fn load(&self, _ctx: &mut LoaderCtx, data: Vec<u8>) -> Result<A> {
        Ok(serde_json::from_slice(&data)?)
    }
}

pub trait Input: Send + Sync + 'static {}

impl<T: Send + Sync + 'static> Input for T {}

#[derive(Clone)]
pub struct AssetLoaderObject {
    ty: TypeId,
    asset_type: TypeId,
    input_type: TypeId,
    loader: Arc<dyn DynAssetLoader>,
}

impl AssetLoaderObject {
    pub fn new<A, L>(loader: L) -> AssetLoaderObject
    where
        A: Asset,
        L: AssetLoader<A>,
    {
        gg_rtti::register::<A>();
        gg_rtti::register::<L>();

        AssetLoaderObject {
            ty: TypeId::of::<L>(),
            asset_type: TypeId::of::<A>(),
            input_type: TypeId::of::<L::Input>(),
            loader: Arc::new((loader, PhantomData::<A>)),
        }
    }

    pub fn ty(&self) -> TypeId {
        self.ty
    }

    pub fn asset_type(&self) -> TypeId {
        self.asset_type
    }

    pub fn input_type(&self) -> TypeId {
        self.input_type
    }

    pub fn filter(&self, input: &dyn SyncAny) -> bool {
        self.loader.filter(input)
    }

    pub async fn load(
        &self,
        ctx: &mut LoaderCtx,
        input: &dyn SyncAny,
    ) -> Result<Box<dyn AnyAsset>> {
        self.loader.load(ctx, input).await
    }
}

impl Debug for AssetLoaderObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetLoaderObject")
            .field("asset_type", &self.asset_type)
            .field("input_type", &self.input_type)
            .finish_non_exhaustive()
    }
}

#[async_trait]
trait DynAssetLoader: Send + Sync {
    fn filter(&self, input: &dyn SyncAny) -> bool;

    async fn load(&self, ctx: &mut LoaderCtx, input: &dyn SyncAny) -> Result<Box<dyn AnyAsset>>;
}

#[async_trait]
impl<A, L> DynAssetLoader for (L, PhantomData<A>)
where
    L: AssetLoader<A>,
    A: Asset,
{
    fn filter(&self, input: &dyn SyncAny) -> bool {
        if let Some(input) = input.downcast_ref::<L::Input>() {
            self.0.filter(input)
        } else {
            false
        }
    }

    async fn load(&self, ctx: &mut LoaderCtx, input: &dyn SyncAny) -> Result<Box<dyn AnyAsset>> {
        if let Some(input) = input.downcast_ref::<L::Input>() {
            let res = self.0.load(ctx, input).await;
            res.map(|v| Box::new(v) as Box<dyn AnyAsset>)
        } else {
            bail!("downcast error")
        }
    }
}
