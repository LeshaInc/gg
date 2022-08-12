use std::marker::PhantomData;

use gg_graphics::Color;

use crate::{Bounds, DrawCtx, View};

pub fn rect<D>(color: impl Into<Color>) -> RectView<D> {
    RectView {
        phantom: PhantomData,
        color: color.into(),
    }
}

pub struct RectView<D> {
    phantom: PhantomData<fn(&mut D)>,
    color: Color,
}

impl<D> View<D> for RectView<D> {
    fn init(&mut self, old: &mut Self) -> bool {
        self.color != old.color
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        ctx.encoder.rect(bounds.rect).fill_color(self.color);
    }
}
