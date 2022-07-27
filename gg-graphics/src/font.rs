use std::cell::RefCell;

use ab_glyph_rasterizer::{point, Point, Rasterizer};
use gg_assets::{Asset, BytesAssetLoader, LoaderCtx, LoaderRegistry};
use gg_math::Vec2;
use gg_util::async_trait;
use gg_util::eyre::{eyre, Result};
use rustybuzz::{Direction, Face, UnicodeBuffer};
pub use ttf_parser::GlyphId;
use ttf_parser::OutlineBuilder;

pub struct Font {
    inner: Inner,
}

#[ouroboros::self_referencing]
struct Inner {
    data: Vec<u8>,
    #[covariant]
    #[borrows(mut data)]
    face: Face<'this>,
}

impl Font {
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
        let ascent = face.ascender() as f32 * scale;
        let descent = face.descender() as f32 * scale;
        let line_gap = face.line_gap() as f32 * scale;
        LineMetrics {
            ascent,
            descent,
            line_gap,
        }
    }

    pub fn rasterize(
        &self,
        glyph: GlyphId,
        size: f32,
        subpixel_offset: SubpixelOffset,
    ) -> Option<GlyphRaster> {
        let offset = subpixel_offset.get();

        let face = self.inner.borrow_face();
        let scale = size / face.units_per_em() as f32;

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

        thread_local! {
            static RASTERIZER: RefCell<Rasterizer> = RefCell::new(Rasterizer::new(64, 64));
        }

        let mut data = vec![0; px_width * px_height];

        RASTERIZER.with(|cell| {
            let mut rasterizer = cell.borrow_mut();
            rasterizer.reset(px_width, px_height);

            face.outline_glyph(
                glyph,
                &mut Outliner {
                    rasterizer: &mut rasterizer,
                    origin: point(px_min.x - offset.x, px_min.y - offset.y),
                    last_move: None,
                    last_pos: point(0.0, 0.0),
                    scale,
                    height: px_height as f32,
                },
            );

            rasterizer.for_each_pixel(|i, a| data[i] = (a * 255.0) as u8);
        });

        Some(GlyphRaster {
            offset: Vec2::new(px_min.x, -px_min.y),
            size: Vec2::new(px_width as u32, px_height as u32),
            data,
        })
    }

    pub fn shape(&self, size: f32, text: &str) -> Vec<ShapedGlyph> {
        let face = self.inner.borrow_face();
        let scale = size / face.units_per_em() as f32;

        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(Direction::LeftToRight);

        let glyphs = rustybuzz::shape(face, &[], buffer);
        let info = glyphs.glyph_infos();
        info.iter()
            .zip(glyphs.glyph_positions())
            .map(|(info, pos)| ShapedGlyph {
                glyph: GlyphId(info.glyph_id as _),
                advance: Vec2::new(pos.x_advance, pos.y_advance).cast::<f32>() * scale,
                offset: Vec2::new(pos.x_offset, pos.y_offset).cast::<f32>() * scale,
            })
            .collect()
    }
}

impl Asset for Font {
    fn register_loaders(registry: &mut LoaderRegistry) {
        registry.add(FontLoader);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LineMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
}

#[derive(Debug)]
pub struct GlyphRaster {
    pub offset: Vec2<f32>,
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

#[derive(Debug)]
pub struct ShapedGlyph {
    pub glyph: GlyphId,
    pub advance: Vec2<f32>,
    pub offset: Vec2<f32>,
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

pub struct FontLoader;

#[async_trait]
impl BytesAssetLoader<Font> for FontLoader {
    async fn load(&self, _: &mut LoaderCtx, bytes: Vec<u8>) -> Result<Font> {
        Ok(Font {
            inner: Inner::try_new(bytes, |bytes| {
                Face::from_slice(bytes, 0).ok_or_else(|| eyre!("font loading error"))
            })?,
        })
    }
}
