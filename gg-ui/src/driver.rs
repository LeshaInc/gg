use gg_assets::Assets;
use gg_graphics::GraphicsEncoder;
use gg_math::{Rect, Vec2};

use crate::{AnyView, DrawCtx, LayoutCtx, View};

pub struct Driver<D> {
    old_view: Option<Box<dyn AnyView<D>>>,
    size: Vec2<f32>,
}

impl<D: 'static> Driver<D> {
    pub fn new() -> Driver<D> {
        Driver {
            old_view: None,
            size: Vec2::zero(),
        }
    }

    pub fn run<V: AnyView<D>>(&mut self, view: V, ctx: UiContext) {
        let mut view: Box<dyn AnyView<D>> = Box::new(view);

        let changed = match self.old_view.take() {
            Some(old) => view.update(&old),
            _ => true,
        };

        if changed || ctx.bounds.extents() != self.size {
            let l_ctx = LayoutCtx { assets: ctx.assets };
            let _hints = view.pre_layout(l_ctx.reborrow());
            self.size = view.layout(l_ctx, ctx.bounds.extents());
        }

        let d_ctx = DrawCtx {
            assets: ctx.assets,
            encoder: ctx.encoder,
        };

        view.draw(d_ctx, Rect::from_pos_extents(ctx.bounds.min, self.size));

        self.old_view = Some(view);
    }
}

pub struct UiContext<'a> {
    pub bounds: Rect<f32>,
    pub assets: &'a Assets,
    pub encoder: &'a mut GraphicsEncoder,
}
