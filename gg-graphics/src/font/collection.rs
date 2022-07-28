use std::path::Path;
use std::sync::Arc;

use gg_assets::{Asset, AssetLoader, Handle, LoaderCtx, LoaderRegistry};
use gg_util::async_trait;
use gg_util::eyre::{bail, Result};
use tracing::error;
use ttf_parser::fonts_in_collection;

use crate::FontFace;

#[derive(Debug)]
pub struct FontCollection {
    pub faces: Vec<Handle<FontFace>>,
}

impl Asset for FontCollection {
    fn register_loaders(registry: &mut LoaderRegistry) {
        registry.add(FontCollectionLoader);
    }
}

pub struct FontCollectionLoader;

#[async_trait]
impl AssetLoader<FontCollection> for FontCollectionLoader {
    type Input = Arc<Path>;

    async fn load(&self, ctx: &mut LoaderCtx, path: &Arc<Path>) -> Result<FontCollection> {
        let bytes = Arc::from(ctx.read_bytes(path)?);
        let num_fonts = fonts_in_collection(&bytes).unwrap_or(1);

        let faces = (0..num_fonts)
            .flat_map(|index| match FontFace::new(bytes.clone(), index) {
                Ok(face) => Some(ctx.insert(face)),
                Err(e) => {
                    error!(path = %path.display(), index, "failed to load font face: {:?}", e);
                    None
                }
            })
            .collect::<Vec<_>>();

        if faces.is_empty() {
            bail!("no valid fonts in collection");
        }

        Ok(FontCollection { faces })
    }
}
