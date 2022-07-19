use std::path::Path;
use std::sync::Arc;

use gg_assets::{Asset, AssetLoader, BytesAssetLoader, Handle, Id, LoaderCtx, LoaderRegistry};
use gg_math::Vec2;
use gg_util::async_trait;
use gg_util::eyre::Result;

#[derive(Clone, Debug)]
pub struct Image {
    pub size: Vec2<u32>,
    pub data: Option<Vec<u8>>,
}

impl Asset for Image {
    fn register_loaders(registry: &mut LoaderRegistry) {
        registry.add(PngLoader);
    }
}

pub struct PngLoader;

#[async_trait]
impl BytesAssetLoader<Image> for PngLoader {
    async fn load(&self, _: &mut LoaderCtx, bytes: Vec<u8>) -> Result<Image> {
        let image = image::load_from_memory(&bytes)?.into_rgba8();
        let size = Vec2::new(image.width(), image.height());
        let data = Some(image.into_flat_samples().samples);
        Ok(Image { size, data })
    }
}

#[derive(Clone, Debug)]
pub struct NinePatchImage {
    pub center: Handle<Image>,
    pub top_left: Handle<Image>,
    pub top: Handle<Image>,
    pub top_right: Handle<Image>,
    pub right: Handle<Image>,
    pub bottom_right: Handle<Image>,
    pub bottom: Handle<Image>,
    pub bottom_left: Handle<Image>,
    pub left: Handle<Image>,
}

impl NinePatchImage {
    pub fn sub_images(&self) -> [Id<Image>; 9] {
        [
            self.center.id(),
            self.top_left.id(),
            self.top.id(),
            self.top_right.id(),
            self.right.id(),
            self.bottom_right.id(),
            self.bottom.id(),
            self.bottom_left.id(),
            self.left.id(),
        ]
    }
}

impl Asset for NinePatchImage {
    fn register_loaders(registry: &mut LoaderRegistry) {
        registry.add(NinePatchImageLoader);
    }
}

pub struct NinePatchImageLoader;

#[async_trait]
impl AssetLoader<NinePatchImage> for NinePatchImageLoader {
    type Input = Arc<Path>;

    async fn load(&self, ctx: &mut LoaderCtx, path: &Arc<Path>) -> Result<NinePatchImage> {
        Ok(NinePatchImage {
            center: ctx.load(path.join("center.png")),
            top_left: ctx.load(path.join("top_left.png")),
            top: ctx.load(path.join("top.png")),
            top_right: ctx.load(path.join("top_right.png")),
            right: ctx.load(path.join("right.png")),
            bottom_right: ctx.load(path.join("bottom_right.png")),
            bottom: ctx.load(path.join("bottom.png")),
            bottom_left: ctx.load(path.join("bottom_left.png")),
            left: ctx.load(path.join("left.png")),
        })
    }
}
