use gg_assets::Assets;
use gg_graphics::{FontDb, GraphicsEncoder, TextLayouter};
use gg_math::{Rect, Vec2};

use crate::Event;

pub trait View<D> {
    fn update(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        let _ = old;
        false
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        let _ = ctx;
        LayoutHints::default()
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        let _ = ctx;
        size
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        let _ = (ctx, bounds);
    }

    fn handle(&mut self, event: Event, data: &mut D) {
        let _ = (event, data);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateResult {
    Changed,
    Unchanged,
}

impl UpdateResult {
    pub fn or(&self, other: UpdateResult) -> UpdateResult {
        use UpdateResult::*;
        match (self, other) {
            (Unchanged, Unchanged) => Unchanged,
            _ => Changed,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutHints {
    pub stretch: f32,
    pub min_size: Vec2<f32>,
    pub max_size: Vec2<f32>,
}

impl Default for LayoutHints {
    fn default() -> Self {
        LayoutHints {
            stretch: 1.0,
            min_size: Vec2::splat(0.0),
            max_size: Vec2::splat(f32::INFINITY),
        }
    }
}

pub struct LayoutCtx<'a> {
    pub assets: &'a Assets,
    pub fonts: &'a FontDb,
    pub text_layouter: &'a mut TextLayouter,
}

impl LayoutCtx<'_> {
    pub fn reborrow(&mut self) -> LayoutCtx<'_> {
        LayoutCtx {
            assets: self.assets,
            fonts: self.fonts,
            text_layouter: self.text_layouter,
        }
    }
}

pub struct DrawCtx<'a> {
    pub assets: &'a Assets,
    pub encoder: &'a mut GraphicsEncoder,
}

impl DrawCtx<'_> {
    pub fn reborrow(&mut self) -> DrawCtx<'_> {
        DrawCtx {
            assets: self.assets,
            encoder: self.encoder,
        }
    }
}
