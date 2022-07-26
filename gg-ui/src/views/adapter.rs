use gg_math::{Rect, Vec2};

use crate::{DrawCtx, Event, LayoutCtx, LayoutHints, View};

pub fn adapter<L, V, D, DV>(lens: L, view: V) -> Adapter<L, V>
where
    L: FnMut(&mut D) -> &mut DV,
    V: View<DV>,
{
    Adapter { view, lens }
}

pub struct Adapter<L, V> {
    lens: L,
    view: V,
}

impl<L, V, D, DV> View<D> for Adapter<L, V>
where
    L: FnMut(&mut D) -> &mut DV,
    V: View<DV>,
{
    fn update(&mut self, old: &Self) -> bool {
        self.view.update(&old.view)
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        self.view.pre_layout(ctx)
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.view.layout(ctx, size)
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        self.view.draw(ctx, bounds)
    }

    fn handle(&mut self, event: Event, data: &mut D) {
        self.view.handle(event, (self.lens)(data))
    }
}
