use gg_math::{Rect, Vec2};

use crate::{DrawCtx, Event, LayoutCtx, LayoutHints, View};

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
    fn update(&mut self, old: &mut Self) -> bool {
        let changed = if self.condition {
            self.view_t.update(&mut old.view_t)
        } else {
            self.view_f.update(&mut old.view_f)
        };

        self.condition == old.condition || changed
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        if self.condition {
            self.view_t.pre_layout(ctx)
        } else {
            self.view_f.pre_layout(ctx)
        }
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        if self.condition {
            self.view_t.layout(ctx, size)
        } else {
            self.view_f.layout(ctx, size)
        }
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        if self.condition {
            self.view_t.draw(ctx, bounds)
        } else {
            self.view_f.draw(ctx, bounds)
        }
    }

    fn handle(&mut self, event: Event, data: &mut D) {
        if self.condition {
            self.view_t.handle(event, data)
        } else {
            self.view_f.handle(event, data)
        }
    }
}
