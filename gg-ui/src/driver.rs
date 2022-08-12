use gg_assets::Assets;
use gg_graphics::{FontDb, GraphicsEncoder, TextLayouter};
use gg_input::Input;
use gg_math::{Rect, Vec2};

use crate::{AnyView, Bounds, DrawCtx, LayoutCtx, UpdateCtx, View};

pub struct Driver<D> {
    old_view: Option<Box<dyn AnyView<D>>>,
    size: Vec2<f32>,
    num_layers: u32,
}

impl<D: 'static> Driver<D> {
    pub fn new() -> Driver<D> {
        Driver {
            old_view: None,
            size: Vec2::zero(),
            num_layers: 1,
        }
    }

    pub fn run<V: AnyView<D>>(&mut self, view: V, ctx: UiContext, data: &mut D) {
        let mut view: Box<dyn AnyView<D>> = Box::new(view);

        let changed = match self.old_view.take() {
            Some(mut old) => view.init(&mut old),
            _ => true,
        };

        if changed || ctx.bounds.size() != self.size {
            let mut l_ctx = LayoutCtx {
                assets: ctx.assets,
                fonts: ctx.fonts,
                text_layouter: ctx.text_layouter,
            };

            let hints = view.pre_layout(&mut l_ctx);
            self.size = view.layout(&mut l_ctx, ctx.bounds.size());
            self.num_layers = hints.num_layers;
        }

        let bounds = Bounds::new(Rect::new(ctx.bounds.min, self.size));

        for layer in 0..self.num_layers {
            for event in ctx.input.events() {
                let mut u_ctx = UpdateCtx {
                    assets: ctx.assets,
                    input: ctx.input,
                    data,
                    layer,
                };

                view.handle(&mut u_ctx, bounds, event);
            }

            let mut d_ctx = DrawCtx {
                assets: ctx.assets,
                text_layouter: ctx.text_layouter,
                encoder: ctx.encoder,
                layer,
            };

            view.draw(&mut d_ctx, bounds);
        }

        self.old_view = Some(view);
    }
}

pub struct UiContext<'a> {
    pub bounds: Rect<f32>,
    pub assets: &'a Assets,
    pub fonts: &'a FontDb,
    pub text_layouter: &'a mut TextLayouter,
    pub encoder: &'a mut GraphicsEncoder,
    pub input: &'a Input,
}
