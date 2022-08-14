use std::sync::atomic::{AtomicBool, Ordering};

use gg_assets::Assets;
use gg_graphics::{FontDb, GraphicsEncoder, TextLayouter};
use gg_input::Input;
use gg_math::{Rect, Vec2};

use crate::{AnyView, Bounds, DrawCtx, LayoutCtx, UiAction, UpdateCtx, View};

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

        let mut bounds = Bounds::new(Rect::new(ctx.bounds.min, self.size));

        let mut u_ctx = UpdateCtx {
            assets: ctx.assets,
            input: ctx.input,
            data,
            dt: ctx.dt,
            layer: 0,
        };

        view.update(&mut u_ctx, bounds);

        for layer in (0..self.num_layers).rev() {
            u_ctx.layer = layer;

            if bounds.hover.is_none() {
                bounds.hover = view.hover(&mut u_ctx, bounds);
            }

            for event in ctx.input.events() {
                view.handle(&mut u_ctx, bounds, event);
            }
        }

        static DEBUG_DRAW: AtomicBool = AtomicBool::new(false);

        let pressed = ctx.input.has_action_pressed(UiAction::DebugDraw);
        let debug_draw = DEBUG_DRAW.fetch_xor(pressed, Ordering::Relaxed) ^ pressed;

        let mut d_ctx = DrawCtx {
            assets: ctx.assets,
            text_layouter: ctx.text_layouter,
            encoder: ctx.encoder,
            layer: 0,
            dt: ctx.dt,
            debug_draw,
        };

        for layer in 0..self.num_layers {
            d_ctx.layer = layer;
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
    pub dt: f32,
}
