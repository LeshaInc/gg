mod asset_set;
mod assets;
mod command;
mod flag;
mod handle;
mod handle_allocator;
mod id;
mod loader;
mod loader_ctx;
mod loaders;
mod metadata;
mod shared;
mod source;
mod storage;
mod sync_any;
mod task;

pub use self::asset_set::{AssetSet, AssetSetAccessor};
pub use self::assets::Assets;
pub use self::handle::{Handle, WeakHandle};
pub use self::id::Id;
pub use self::loader::{AssetLoader, BytesAssetLoader, Input, JsonAssetLoader};
pub use self::loader_ctx::LoaderCtx;
pub use self::loaders::LoaderRegistry;
pub use self::source::{DirSource, Source};

pub trait Asset: Send + Sync + 'static {
    fn register_loaders(registry: &mut LoaderRegistry) {
        let _ = registry;
    }
}
