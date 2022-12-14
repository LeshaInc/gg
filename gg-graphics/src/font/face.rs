use std::sync::Arc;

use ab_glyph_rasterizer::{point, Point, Rasterizer};
use gg_assets::Asset;
use gg_math::{Rect, Vec2};
use gg_util::eyre::{eyre, Result};
use image::imageops::FilterType;
use rustybuzz::{Direction, Face, UnicodeBuffer};
pub use ttf_parser::os2::{Style as FontStyle, Weight as FontWeight};
pub use ttf_parser::GlyphId;
use ttf_parser::OutlineBuilder;

pub struct FontFace {
    inner: Inner,
    props: FontFaceProps,
}

#[derive(Clone, Debug)]
pub struct FontFaceProps {
    pub name: String,
    pub weight: FontWeight,
    pub style: FontStyle,
}

#[ouroboros::self_referencing]
struct Inner {
    data: Arc<[u8]>,
    #[covariant]
    #[borrows(data)]
    face: Face<'this>,
}

impl FontFace {
    pub fn new(data: Arc<[u8]>, index: u32) -> Result<FontFace> {
        let inner = Inner::try_new(data, |data| {
            Face::from_slice(data, index).ok_or_else(|| eyre!("font parsing error"))
        })?;

        let face = inner.borrow_face();

        let raw = face.names().get(1);
        let name = raw
            .and_then(|v| v.to_string())
            .ok_or_else(|| eyre!("font does not have a name"))?;

        let props = FontFaceProps {
            name,
            weight: face.weight(),
            style: face.style(),
        };

        Ok(FontFace { inner, props })
    }

    pub fn props(&self) -> &FontFaceProps {
        &self.props
    }

    pub fn lookup_glyph(&self, ch: char) -> GlyphId {
        let face = self.inner.borrow_face();
        face.glyph_index(ch).unwrap_or(GlyphId(0))
    }

    pub fn glyph_advance(&self, glyph: GlyphId, size: f32) -> f32 {
        let face = self.inner.borrow_face();
        let scale = size / face.units_per_em() as f32;
        face.glyph_hor_advance(glyph)
            .map(|v| v as f32 * scale)
            .unwrap_or(0.0)
    }

    pub fn line_metrics(&self, size: f32) -> LineMetrics {
        let face = self.inner.borrow_face();
        let scale = size / face.units_per_em() as f32;
        LineMetrics {
            ascender: face.ascender() as f32 * scale,
            descender: face.descender() as f32 * scale,
            line_gap: face.line_gap() as f32 * scale,
        }
    }

    pub fn rasterize(
        &self,
        cache: &mut RasterizationCache,
        glyph: GlyphId,
        size: f32,
        subpixel_offset: SubpixelOffset,
    ) -> Option<GlyphRaster> {
        let face = self.inner.borrow_face();
        let scale = size / face.units_per_em() as f32;

        let offset = subpixel_offset.get();
        let bbox = face.glyph_bounding_box(glyph)?;
        let px_min =
            (Vec2::new((bbox.x_min as f32) * scale, (bbox.y_min as f32) * scale) + offset).floor();
        let px_max =
            (Vec2::new((bbox.x_max as f32) * scale, (bbox.y_max as f32) * scale) + offset).ceil();

        let px_width = (px_max.x - px_min.x).max(0.0) as usize;
        let px_height = (px_max.y - px_min.y).max(0.0) as usize;
        if px_width == 0 || px_height == 0 {
            return None;
        }

        let mut data = vec![0; px_width * px_height];
        cache.rasterizer.reset(px_width, px_height);

        face.outline_glyph(
            glyph,
            &mut Outliner {
                rasterizer: &mut cache.rasterizer,
                origin: point(px_min.x - offset.x, px_min.y - offset.y),
                last_move: None,
                last_pos: point(0.0, 0.0),
                scale,
                height: px_height as f32,
            },
        );

        cache
            .rasterizer
            .for_each_pixel(|i, a| data[i] = (a * 255.0) as u8);

        let raster_size = Vec2::new(px_width, px_height).cast::<u32>();

        Some(GlyphRaster {
            bounds: Rect::new(
                Vec2::new(px_min.x, -px_min.y) / size,
                raster_size.cast::<f32>() / size,
            ),
            size: raster_size,
            data,
        })
    }

    pub fn has_image(&self, glyph: GlyphId) -> bool {
        let face = self.inner.borrow_face();
        face.glyph_raster_image(glyph, u16::MAX).is_some()
    }

    pub fn get_image(&self, glyph: GlyphId, size: u32) -> Option<GlyphRaster> {
        let face = self.inner.borrow_face();

        let raster = match face.glyph_raster_image(glyph, size.min(u16::MAX.into()) as u16) {
            Some(v) => v,
            None => return None,
        };

        let scale = raster.pixels_per_em as f32;

        let mut image = image::load_from_memory(raster.data).ok()?.into_rgba8();

        let old_size = Vec2::new(image.width(), image.height());
        let size = (old_size.cast::<f32>() / scale * (size as f32)).cast::<u32>();

        if size.cmp_lt(old_size).any() {
            image = image::imageops::resize(&image, size.x, size.y, FilterType::Triangle);
        }

        Some(GlyphRaster {
            bounds: Rect::new(
                Vec2::new(raster.x, -raster.y).cast::<f32>() / scale,
                Vec2::new(raster.width, raster.height).cast::<f32>() / scale,
            ),
            size: Vec2::new(image.width(), image.height()),
            data: image.into_flat_samples().samples,
        })
    }

    pub fn shape(
        &self,
        cache: &mut ShapingCache,
        size: f32,
        text: &str,
        buf: &mut Vec<ShapedGlyph>,
    ) {
        let face = self.inner.borrow_face();
        let scale = size / face.units_per_em() as f32;

        let mut buffer = std::mem::take(&mut cache.buffer);
        buffer.push_str(text);
        buffer.set_direction(Direction::LeftToRight);

        let glyphs = rustybuzz::shape(face, &[], buffer);
        let it = glyphs.glyph_infos().iter().zip(glyphs.glyph_positions());
        buf.extend(it.map(|(info, pos)| ShapedGlyph {
            glyph: GlyphId(info.glyph_id as _),
            advance: Vec2::new(pos.x_advance, pos.y_advance).cast::<f32>() * scale,
            offset: Vec2::new(pos.x_offset, pos.y_offset).cast::<f32>() * scale,
            cluster: info.cluster,
        }));

        cache.buffer = glyphs.clear();
    }
}

impl Asset for FontFace {}

#[derive(Clone, Copy, Debug)]
pub struct LineMetrics {
    pub ascender: f32,
    pub descender: f32,
    pub line_gap: f32,
}

#[derive(Debug)]
pub struct GlyphRaster {
    pub bounds: Rect<f32>,
    pub size: Vec2<u32>,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SubpixelOffset {
    sx: u8,
    sy: u8,
}

impl SubpixelOffset {
    pub fn new(frac: Vec2<f32>) -> SubpixelOffset {
        SubpixelOffset {
            sx: (frac.x * 4.0) as u8,
            sy: (frac.y * 2.0) as u8,
        }
    }

    pub fn get(self) -> Vec2<f32> {
        Vec2::new((self.sx as f32) / 4.0, (self.sy as f32) / 2.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    pub glyph: GlyphId,
    pub advance: Vec2<f32>,
    pub offset: Vec2<f32>,
    pub cluster: u32,
}

#[derive(Debug)]
pub struct RasterizationCache {
    rasterizer: Rasterizer,
}

impl Default for RasterizationCache {
    fn default() -> Self {
        Self {
            rasterizer: Rasterizer::new(64, 64),
        }
    }
}

#[derive(Debug, Default)]
pub struct ShapingCache {
    buffer: UnicodeBuffer,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ShapingCacheKey {
    font_data: usize,
    font_index: u32,
    size: u32,
}

struct Outliner<'a> {
    rasterizer: &'a mut Rasterizer,
    origin: Point,
    last_move: Option<Point>,
    last_pos: Point,
    scale: f32,
    height: f32,
}

impl Outliner<'_> {
    fn scale(&self, x: f32, y: f32) -> Point {
        point(
            x * self.scale - self.origin.x,
            self.height - y * self.scale + self.origin.y,
        )
    }
}

impl OutlineBuilder for Outliner<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let pos = self.scale(x, y);
        self.last_pos = pos;
        self.last_move = Some(pos);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let end = self.scale(x, y);
        self.rasterizer.draw_line(self.last_pos, end);
        self.last_pos = end;
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let c0 = self.scale(x1, y1);
        let end = self.scale(x, y);
        self.rasterizer.draw_quad(self.last_pos, c0, end);
        self.last_pos = end;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let c0 = self.scale(x1, y1);
        let c1 = self.scale(x2, y2);
        let end = self.scale(x, y);
        self.rasterizer.draw_cubic(self.last_pos, c0, c1, end);
        self.last_pos = end;
    }

    fn close(&mut self) {
        if let Some(pos) = self.last_move.take() {
            self.rasterizer.draw_line(self.last_pos, pos);
        }
    }
}
