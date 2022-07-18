use gg_assets::{Handle, Id};
use gg_math::{Affine2, Rect, Vec2};

use crate::{Canvas, Color, Font, GlyphIndex, Image, NinePatchImage};

#[derive(Debug)]
pub struct CommandList {
    pub canvas: Canvas,
    pub list: Vec<Command>,
}

#[derive(Clone, Debug)]
pub enum Command {
    Save,
    Restore,
    SetScissor(Rect<u32>),
    ClearScissor,
    PreTransform(Affine2<f32>),
    PostTransform(Affine2<f32>),
    DrawRect(DrawRect),
    DrawGlyph(DrawGlyph),
}

impl From<DrawRect> for Command {
    fn from(cmd: DrawRect) -> Self {
        Command::DrawRect(cmd)
    }
}

#[derive(Clone, Debug)]
pub struct DrawRect {
    pub rect: Rect<f32>,
    pub fill: Fill,
}

#[derive(Clone, Copy, Debug)]
pub struct DrawGlyph {
    pub font: Id<Font>,
    pub glyph: GlyphIndex,
    pub size: f32,
    pub pos: Vec2<f32>,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct Fill {
    pub color: Color,
    pub image: Option<FillImage>,
}

#[derive(Clone, Debug)]
pub enum FillImage {
    Canvas(Canvas),
    SingleImage(Id<Image>),
    NinePatchImage(Id<NinePatchImage>),
}

impl From<Canvas> for FillImage {
    fn from(canvas: Canvas) -> Self {
        FillImage::Canvas(canvas)
    }
}

impl From<&Canvas> for FillImage {
    fn from(canvas: &Canvas) -> Self {
        FillImage::Canvas(canvas.clone())
    }
}

impl From<Id<Image>> for FillImage {
    fn from(id: Id<Image>) -> Self {
        FillImage::SingleImage(id)
    }
}

impl From<Id<NinePatchImage>> for FillImage {
    fn from(id: Id<NinePatchImage>) -> Self {
        FillImage::NinePatchImage(id)
    }
}

impl From<&Handle<Image>> for FillImage {
    fn from(handle: &Handle<Image>) -> Self {
        FillImage::SingleImage(handle.id())
    }
}

impl From<&Handle<NinePatchImage>> for FillImage {
    fn from(handle: &Handle<NinePatchImage>) -> Self {
        FillImage::NinePatchImage(handle.id())
    }
}
