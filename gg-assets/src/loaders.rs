use gg_util::ahash::AHashMap;
use gg_util::rtti::TypeId;
use smallvec::SmallVec;

use crate::loader::AssetLoaderObject;
use crate::{Asset, AssetLoader};

#[derive(Debug, Default)]
pub struct AssetLoaders {
    loaders: AHashMap<TypeId, AssetLoaderObject>,
    mapping: AHashMap<MappingKey, SmallVec<[TypeId; 1]>>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MappingKey {
    asset_type: TypeId,
    input_type: TypeId,
}

impl AssetLoaders {
    pub fn new() -> AssetLoaders {
        AssetLoaders::default()
    }

    pub fn insert_asset_loaders<A: Asset>(&mut self) {
        let loaders = std::mem::take(self);
        let mut registry = LoaderRegistry { loaders };
        A::register_loaders(&mut registry);
        *self = registry.loaders;
    }

    pub fn insert(&mut self, loader: AssetLoaderObject) {
        let key = MappingKey {
            asset_type: loader.asset_type(),
            input_type: loader.input_type(),
        };

        self.mapping.entry(key).or_default().push(loader.ty());
        self.loaders.insert(loader.ty(), loader);
    }

    pub fn get(&self, ty: TypeId) -> &AssetLoaderObject {
        &self.loaders[&ty]
    }

    pub fn lookup(&self, asset_type: TypeId, input_type: TypeId) -> &[TypeId] {
        let key = MappingKey {
            asset_type,
            input_type,
        };

        self.mapping.get(&key).map(|l| l.as_slice()).unwrap_or(&[])
    }
}

pub struct LoaderRegistry {
    loaders: AssetLoaders,
}

impl LoaderRegistry {
    pub fn add<A: Asset, L: AssetLoader<A>>(&mut self, loader: L) {
        gg_util::rtti::register::<L>();
        self.loaders.insert(AssetLoaderObject::new(loader));
    }
}
