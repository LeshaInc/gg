use std::hash::{Hash, Hasher};

use gg_assets::{Assets, Id};
use gg_graphics::{Font, GlyphIndex, GlyphMetrics};
use gg_math::Rect;
use gg_util::ahash::AHashMap;
use wgpu::TextureFormat;

use crate::atlas::{AtlasId, AtlasPool, PoolAllocation, PoolImage};

#[derive(Debug, Default)]
pub struct Glyphs {
    map: AHashMap<GlyphKey, Option<(GlyphMetrics, PoolAllocation)>>,
}

impl Glyphs {
    pub fn new() -> Glyphs {
        Glyphs::default()
    }

    pub fn get(
        &self,
        atlases: &AtlasPool,
        key: GlyphKey,
    ) -> Option<(GlyphMetrics, AtlasId, Rect<f32>)> {
        let entry = *self.map.get(&key)?;
        let (metrics, alloc) = entry?;
        let rect = atlases.get_normalized_rect(&alloc);
        Some((metrics, alloc.id.atlas_id, rect))
    }

    pub fn alloc(&mut self, atlases: &mut AtlasPool, assets: &Assets, key: GlyphKey) {
        if self.map.contains_key(&key) {
            return;
        }

        let font = match assets.get_by_id(key.font) {
            Some(v) => v,
            None => return,
        };

        let (metrics, data) = font.rasterize(key.glyph, key.size);
        if data.is_empty() {
            self.map.insert(key, None);
        } else {
            let alloc = atlases.alloc(PoolImage {
                size: metrics.bitmap_size(),
                data,
                format: TextureFormat::R8Unorm,
                preferred_allocator: None,
            });

            self.map.insert(key, Some((metrics, alloc)));
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphKey {
    pub font: Id<Font>,
    pub glyph: GlyphIndex,
    pub size: f32,
}

impl PartialEq for GlyphKey {
    fn eq(&self, rhs: &GlyphKey) -> bool {
        self.font == rhs.font
            && self.glyph == rhs.glyph
            && self.size.to_bits() == rhs.size.to_bits()
    }
}

impl Eq for GlyphKey {}

impl Hash for GlyphKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.font.hash(state);
        self.glyph.hash(state);
        self.size.to_bits().hash(state);
    }
}
