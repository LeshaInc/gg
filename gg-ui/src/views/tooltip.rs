use gg_math::{Rect, Vec2};

use crate::{Bounds, DrawCtx, Event, Hover, LayoutCtx, LayoutHints, UpdateCtx, View};

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
    fn init(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        self.view_layers = old.view_layers;
        self.size = old.size;

        self.view.init(&mut old.view) | self.contents.init(&mut old.contents)
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        let view_hints = self.view.pre_layout(ctx);
        let contents_hints = self.contents.pre_layout(ctx);

        self.view_layers = view_hints.num_layers;
        self.size = contents_hints.min_size;

        LayoutHints {
            num_layers: self.view_layers + contents_hints.num_layers,
            ..view_hints
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.size = self.contents.layout(ctx, self.size);
        self.view.layout(ctx, size)
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        if ctx.layer < self.view_layers {
            self.view.hover(ctx, bounds)
        } else {
            let mut ctx = ctx.reborrow();
            ctx.layer -= self.view_layers;

            let bounds = Bounds::new(Rect::new(
                Vec2::new(bounds.rect.min.x, bounds.rect.max.y),
                self.size,
            ));

            self.contents.hover(&mut ctx, bounds)
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        if ctx.layer < self.view_layers {
            self.view.update(ctx, bounds)
        } else {
            let mut ctx = ctx.reborrow();
            ctx.layer -= self.view_layers;

            let bounds = Bounds::new(Rect::new(
                Vec2::new(bounds.rect.min.x, bounds.rect.max.y),
                self.size,
            ));

            self.contents.update(&mut ctx, bounds)
        }
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        if ctx.layer < self.view_layers {
            self.view.handle(ctx, bounds, event)
        } else {
            let mut ctx = ctx.reborrow();
            ctx.layer -= self.view_layers;

            let bounds = Bounds::new(Rect::new(
                Vec2::new(bounds.rect.min.x, bounds.rect.max.y),
                self.size,
            ));

            self.contents.handle(&mut ctx, bounds, event)
        }
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        if ctx.layer < self.view_layers {
            self.view.draw(ctx, bounds)
        } else {
            let mut ctx = ctx.reborrow();
            ctx.layer -= self.view_layers;

            let bounds = Bounds::new(Rect::new(
                Vec2::new(bounds.rect.min.x, bounds.rect.max.y),
                self.size,
            ));

            self.contents.draw(&mut ctx, bounds)
        }
    }
}
