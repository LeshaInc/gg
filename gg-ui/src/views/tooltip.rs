use gg_math::{Rect, Vec2};

use crate::{Bounds, DrawCtx, Event, HandleCtx, LayoutCtx, LayoutHints, View};

pub fn tooltip<V, VT>(view: V, contents: VT) -> Tooltip<V, VT> {
    Tooltip {
        view,
        view_layers: 0,
        contents,
        size: Vec2::zero(),
    }
}

pub struct Tooltip<V, VT> {
    view: V,
    contents: VT,
    view_layers: u32,
    size: Vec2<f32>,
}

impl<D, V, VT> View<D> for Tooltip<V, VT>
where
    V: View<D>,
    VT: View<D>,
{
    fn update(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        self.view_layers = old.view_layers;
        self.size = old.size;

        self.view.update(&mut old.view) | self.contents.update(&mut old.contents)
    }

    fn pre_layout(&mut self, mut ctx: LayoutCtx) -> LayoutHints {
        let view_hints = self.view.pre_layout(ctx.reborrow());
        let contents_hints = self.contents.pre_layout(ctx);

        self.view_layers = view_hints.num_layers;
        self.size = contents_hints.min_size;

        LayoutHints {
            num_layers: self.view_layers + contents_hints.num_layers,
            ..view_hints
        }
    }

    fn layout(&mut self, mut ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.size = self.contents.layout(ctx.reborrow(), self.size);
        self.view.layout(ctx, size)
    }

    fn draw(&mut self, mut ctx: DrawCtx, bounds: Bounds) {
        if ctx.layer < self.view_layers {
            self.view.draw(ctx, bounds)
        } else {
            ctx.layer -= self.view_layers;
            let bounds = Bounds {
                rect: Rect::new(Vec2::new(bounds.rect.min.x, bounds.rect.max.y), self.size),
                scissor: Rect::new(Vec2::zero(), Vec2::splat(f32::INFINITY)),
            };
            self.contents.draw(ctx, bounds)
        }
    }

    fn handle(&mut self, mut ctx: HandleCtx<D>, bounds: Bounds, event: Event) {
        if ctx.layer < self.view_layers {
            self.view.handle(ctx, bounds, event)
        } else {
            ctx.layer -= self.view_layers;
            let bounds = Bounds {
                rect: Rect::new(Vec2::new(bounds.rect.min.x, bounds.rect.max.y), self.size),
                scissor: Rect::new(Vec2::zero(), Vec2::splat(f32::INFINITY)),
            };
            self.contents.handle(ctx, bounds, event)
        }
    }
}
