use std::marker::PhantomData;

use gg_graphics::Color;
use gg_math::Rect;

use crate::{DrawCtx, View};

pub fn rect<D>(color: impl Into<Color>) -> RectView<D> {
    RectView {
        phantom: PhantomData,
        color: color.into(),
    }
}

pub struct RectView<D> {
    phantom: PhantomData<fn(D)>,
    color: Color,
}

impl<D> View<D> for RectView<D> {
    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        ctx.encoder.rect(bounds).fill_color(self.color);
    }
}
