use gg_math::{Rect, Vec2};

use crate::{Bounds, DrawCtx, Event, HandleCtx, LayoutCtx, LayoutHints, View};

pub fn tooltip<V, VT>(view: V, contents: VT) -> Tooltip<V, VT> {
    Tooltip {
        view,
        contents,
        size: Vec2::zero(),
    }
}

pub struct Tooltip<V, VT> {
    view: V,
    contents: VT,
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
        self.view.update(&mut old.view) | self.contents.update(&mut old.contents)
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        let mut hints = self.view.pre_layout(ctx);
        hints.num_layers += 1;
        hints
    }

    fn layout(&mut self, mut ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        let hints = self.contents.pre_layout(ctx.reborrow());
        self.size = self.contents.layout(ctx.reborrow(), hints.min_size);

        self.view.layout(ctx, size)
    }

    fn draw(&mut self, mut ctx: DrawCtx, bounds: Bounds) {
        if ctx.layer == 0 {
            self.view.draw(ctx, bounds)
        } else {
            ctx.layer -= 1;
            let bounds = Bounds {
                rect: Rect::new(Vec2::new(bounds.rect.min.x, bounds.rect.max.y), self.size),
                scissor: Rect::new(Vec2::zero(), Vec2::splat(f32::INFINITY)),
            };
            self.contents.draw(ctx, bounds)
        }
    }

    fn handle(&mut self, mut ctx: HandleCtx<D>, bounds: Bounds, event: Event) {
        if ctx.layer == 0 {
            self.view.handle(ctx, bounds, event)
        } else {
            ctx.layer -= 1;
            let bounds = Bounds {
                rect: Rect::new(Vec2::new(bounds.rect.min.x, bounds.rect.max.y), self.size),
                scissor: Rect::new(Vec2::zero(), Vec2::splat(f32::INFINITY)),
            };
            self.contents.handle(ctx, bounds, event)
        }
    }
}
