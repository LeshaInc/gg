use gg_math::{Rect, SideOffsets, Vec2};

use crate::{AppendChild, DrawCtx, Event, HandleCtx, LayoutCtx, LayoutHints, View};

pub fn padding<O, V>(offsets: O, view: V) -> Padding<V>
where
    O: Into<SideOffsets<f32>>,
{
    Padding {
        view,
        offsets: offsets.into(),
    }
}

pub struct Padding<V> {
    view: V,
    offsets: SideOffsets<f32>,
}

impl<D, V, VC> AppendChild<D, VC> for Padding<V>
where
    V: View<D> + AppendChild<D, VC>,
    VC: View<D>,
{
    type Output = Padding<V::Output>;

    fn child(self, child: VC) -> Self::Output {
        Padding {
            view: self.view.child(child),
            offsets: self.offsets,
        }
    }
}

impl<D, V: View<D>> View<D> for Padding<V> {
    fn update(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        (self.offsets != old.offsets) | self.view.update(&mut old.view)
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        let mut hints = self.view.pre_layout(ctx);
        hints.min_size += self.offsets.size();
        hints.max_size += self.offsets.size();
        hints
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        self.view.layout(ctx, size - self.offsets.size()) + self.offsets.size()
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        self.view.draw(ctx, bounds.shrink(&self.offsets));
    }

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event) {
        self.view.handle(ctx, bounds.shrink(&self.offsets), event);
    }
}
