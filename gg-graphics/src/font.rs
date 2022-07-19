use async_trait::async_trait;
use eyre::{eyre, Result};
use fontdue::FontSettings;
use gg_assets::{Asset, BytesAssetLoader, LoaderCtx, LoaderRegistry};
use gg_math::Vec2;

#[derive(Clone, Debug)]
pub struct Font {
    inner: fontdue::Font,
}

impl Font {
    pub fn lookup_glyph(&self, ch: char) -> GlyphIndex {
        GlyphIndex(self.inner.lookup_glyph_index(ch))
    }

    pub fn kern(&self, left: GlyphIndex, right: GlyphIndex, size: f32) -> Option<f32> {
        self.inner.horizontal_kern_indexed(left.0, right.0, size)
    }

    pub fn glyph_metrics(&self, glyph: GlyphIndex, size: f32) -> GlyphMetrics {
        let inner = self.inner.metrics_indexed(glyph.0, size);
        GlyphMetrics { inner }
    }

    pub fn line_metrics(&self, size: f32) -> Option<LineMetrics> {
        let inner = self.inner.horizontal_line_metrics(size)?;
        Some(LineMetrics { inner })
    }

    pub fn rasterize(&self, glyph: GlyphIndex, size: f32) -> (GlyphMetrics, Vec<u8>) {
        let (metrics, data) = self.inner.rasterize_indexed(glyph.0, size);
        (GlyphMetrics { inner: metrics }, data)
    }
}

impl Asset for Font {
    fn register_loaders(registry: &mut LoaderRegistry) {
        registry.add(FontLoader);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GlyphIndex(u16);

#[derive(Clone, Copy, Debug)]
pub struct GlyphMetrics {
    inner: fontdue::Metrics,
}

impl GlyphMetrics {
    pub fn bitmap_offset(&self) -> Vec2<i32> {
        Vec2::new(self.inner.xmin, -self.inner.ymin)
    }

    pub fn bitmap_size(&self) -> Vec2<u32> {
        Vec2::new(self.inner.width, self.inner.height).cast()
    }

    pub fn advance(&self) -> f32 {
        self.inner.advance_width
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LineMetrics {
    inner: fontdue::LineMetrics,
}

impl LineMetrics {
    pub fn ascent(&self) -> f32 {
        self.inner.ascent
    }

    pub fn descent(&self) -> f32 {
        self.inner.descent
    }

    pub fn line_gap(&self) -> f32 {
        self.inner.line_gap
    }
}

pub struct FontLoader;

#[async_trait]
impl BytesAssetLoader<Font> for FontLoader {
    async fn load(&self, _: &mut LoaderCtx, bytes: Vec<u8>) -> Result<Font> {
        let inner = fontdue::Font::from_bytes(bytes, FontSettings::default())
            .map_err(|e| eyre!("font error: {}", e))?;
        Ok(Font { inner })
    }
}
