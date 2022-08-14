use gg_math::Vec2;

use crate::{Bounds, DrawCtx, Event, LayoutCtx, LayoutHints, UpdateCtx, View};

pub fn choose<VT, VF>(condition: bool, view_t: VT, view_f: VF) -> Choice<VT, VF> {
    Choice {
        view_t,
        view_f,
        condition,
    }
}

pub struct Choice<VT, VF> {
    view_t: VT,
    view_f: VF,
    condition: bool,
}

impl<D, VT, VF> View<D> for Choice<VT, VF>
where
    VT: View<D>,
    VF: View<D>,
{
    fn init(&mut self, old: &mut Self) -> bool {
        let changed = if self.condition {
            self.view_t.init(&mut old.view_t)
        } else {
            self.view_f.init(&mut old.view_f)
        };

        self.condition == old.condition || changed
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        if self.condition {
            self.view_t.pre_layout(ctx)
        } else {
            self.view_f.pre_layout(ctx)
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        if self.condition {
            self.view_t.layout(ctx, size)
        } else {
            self.view_f.layout(ctx, size)
        }
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) -> bool {
        if self.condition {
            self.view_t.handle(ctx, bounds, event)
        } else {
            self.view_f.handle(ctx, bounds, event)
        }
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        if self.condition {
            self.view_t.draw(ctx, bounds)
        } else {
            self.view_f.draw(ctx, bounds)
        }
    }
}
