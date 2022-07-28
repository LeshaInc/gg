use gg_assets::{Assets, Id};
use gg_graphics::{FontFace, GlyphId, SubpixelOffset};
use gg_math::{Rect, Vec2};
use gg_util::ahash::AHashMap;
use wgpu::TextureFormat;

use crate::atlas::{AtlasPool, PoolAllocation, PoolImage};

#[derive(Debug, Default)]
pub struct Glyphs {
    map: AHashMap<GlyphKey, Option<Glyph>>,
}

#[derive(Copy, Clone, Debug)]
pub struct Glyph {
    pub bounds: Rect<f32>,
    pub size: Vec2<u32>,
    pub alloc: PoolAllocation,
    pub is_image: bool,
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

        let res = match key.kind {
            GlyphKeyKind::Image { size } => font
                .get_image(key.glyph, size)
                .map(|raster| (raster, TextureFormat::Rgba8UnormSrgb)),
            GlyphKeyKind::Vector {
                size,
                subpixel_offset,
            } => font
                .rasterize(key.glyph, f32::from_bits(size), subpixel_offset)
                .map(|raster| (raster, TextureFormat::R8Unorm)),
        };

        let (raster, format) = match res {
            Some(v) => v,
            None => {
                self.map.insert(key, None);
                return;
            }
        };

        let alloc = atlases.alloc(PoolImage {
            size: raster.size,
            data: raster.data,
            format,
            preferred_allocator: None,
        });

        let glyph = Glyph {
            bounds: raster.bounds,
            size: raster.size,
            alloc,
            is_image: format == TextureFormat::Rgba8UnormSrgb,
        };

        self.map.insert(key, Some(glyph));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct GlyphKey {
    pub font: Id<FontFace>,
    pub glyph: GlyphId,
    pub kind: GlyphKeyKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum GlyphKeyKind {
    Vector {
        size: u32,
        subpixel_offset: SubpixelOffset,
    },
    Image {
        size: u32,
    },
}
