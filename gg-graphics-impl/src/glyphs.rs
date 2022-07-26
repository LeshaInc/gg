use std::hash::{Hash, Hasher};

use gg_assets::{Assets, Id};
use gg_graphics::{Font, GlyphId};
use gg_math::Vec2;
use gg_util::ahash::AHashMap;
use wgpu::TextureFormat;

use crate::atlas::{AtlasPool, PoolAllocation, PoolImage};

#[derive(Debug, Default)]
pub struct Glyphs {
    map: AHashMap<GlyphKey, Option<Glyph>>,
}

#[derive(Copy, Clone, Debug)]
pub struct Glyph {
    pub offset: Vec2<f32>,
    pub size: Vec2<u32>,
    pub alloc: PoolAllocation,
}

impl Glyphs {
    pub fn new() -> Glyphs {
        Glyphs::default()
    }

    pub fn get(&self, key: GlyphKey) -> Option<Glyph> {
        *self.map.get(&key)?
    }

    pub fn alloc(&mut self, atlases: &mut AtlasPool, assets: &Assets, key: GlyphKey) {
        if self.map.contains_key(&key) {
            return;
        }

        let font = match assets.get_by_id(key.font) {
            Some(v) => v,
            None => return,
        };

        if let Some(raster) = font.rasterize(key.glyph, key.size) {
            let alloc = atlases.alloc(PoolImage {
                size: raster.size,
                data: raster.data,
                format: TextureFormat::R8Unorm,
                preferred_allocator: None,
            });

            let glyph = Glyph {
                offset: raster.offset,
                size: raster.size,
                alloc,
            };

            self.map.insert(key, Some(glyph));
        } else {
            self.map.insert(key, None);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphKey {
    pub font: Id<Font>,
    pub glyph: GlyphId,
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
