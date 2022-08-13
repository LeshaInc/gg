use gg_assets::Assets;
use gg_graphics::{FontDb, GraphicsEncoder, TextLayouter};
use gg_input::Input;
use gg_math::{Rect, Vec2};

use crate::Event;

pub trait View<D> {
    fn init(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        let _ = old;
        false
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        let _ = ctx;
        LayoutHints::default()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        let _ = ctx;
        size
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        if bounds.clip_rect.contains(ctx.input.mouse_pos()) {
            Hover::Direct
        } else {
            Hover::None
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        let _ = (ctx, bounds);
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        let _ = (ctx, bounds, event);
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        let _ = (ctx, bounds);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutHints {
    pub stretch: f32,
    pub min_size: Vec2<f32>,
    pub max_size: Vec2<f32>,
    pub num_layers: u32,
}

impl Default for LayoutHints {
    fn default() -> Self {
        LayoutHints {
            stretch: 0.0,
            min_size: Vec2::splat(0.0),
            max_size: Vec2::splat(f32::INFINITY),
            num_layers: 1,
        }
    }
}

pub struct LayoutCtx<'a> {
    pub assets: &'a Assets,
    pub fonts: &'a FontDb,
    pub text_layouter: &'a mut TextLayouter,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Hover {
    None,
    Indirect,
    Direct,
}

impl Default for Hover {
    fn default() -> Hover {
        Hover::None
    }
}

impl Hover {
    pub fn is_none(self) -> bool {
        self == Hover::None
    }

    pub fn is_some(self) -> bool {
        self != Hover::None
    }

    pub fn is_direct(self) -> bool {
        self == Hover::Direct
    }

    pub fn is_indirect(self) -> bool {
        self == Hover::Indirect
    }
}

pub struct UpdateCtx<'a, D> {
    pub assets: &'a Assets,
    pub input: &'a Input,
    pub data: &'a mut D,
    pub layer: u32,
}

impl<D> UpdateCtx<'_, D> {
    pub fn reborrow(&mut self) -> UpdateCtx<'_, D> {
        UpdateCtx {
            assets: self.assets,
            input: self.input,
            data: self.data,
            layer: self.layer,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub rect: Rect<f32>,
    pub clip_rect: Rect<f32>,
    pub scissor: Rect<f32>,
    pub hover: Hover,
}

impl Bounds {
    pub fn new(rect: Rect<f32>) -> Bounds {
        Bounds {
            rect,
            clip_rect: rect,
            scissor: Rect::new(Vec2::zero(), Vec2::splat(f32::INFINITY)),
            hover: Hover::None,
        }
    }

    pub fn with_scissor(mut self, scissor: Rect<f32>) -> Bounds {
        self.scissor = scissor;
        self
    }

    pub fn child(&self, rect: Rect<f32>, hover: Hover) -> Bounds {
        Bounds {
            rect,
            clip_rect: rect.f_intersect(&self.scissor),
            scissor: self.scissor,
            hover,
        }
    }
}

pub struct DrawCtx<'a> {
    pub assets: &'a Assets,
    pub text_layouter: &'a mut TextLayouter,
    pub encoder: &'a mut GraphicsEncoder,
    pub layer: u32,
    pub debug_draw: bool,
}

impl DrawCtx<'_> {
    pub fn reborrow(&mut self) -> DrawCtx<'_> {
        DrawCtx {
            assets: self.assets,
            text_layouter: self.text_layouter,
            encoder: self.encoder,
            layer: self.layer,
            debug_draw: self.debug_draw,
        }
    }
}
