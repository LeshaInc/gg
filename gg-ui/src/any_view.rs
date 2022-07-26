use std::any::Any;

use gg_math::{Rect, Vec2};

use crate::{DrawCtx, Event, LayoutCtx, LayoutHints, View};

pub trait AnyView<D: 'static>: Any + View<D> {
    fn as_any(&self) -> &dyn Any;

    fn update_dyn(&mut self, old: &dyn AnyView<D>) -> bool;
}

impl<D: 'static, V: Any + View<D>> AnyView<D> for V {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn update_dyn(&mut self, old: &dyn AnyView<D>) -> bool {
        if let Some(old) = old.as_any().downcast_ref::<Self>() {
            self.update(old)
        } else {
            true
        }
    }
}

impl<D: 'static> View<D> for Box<dyn AnyView<D>> {
    fn update(&mut self, old: &Self) -> bool {
        (**self).update_dyn(&**old)
    }

    fn pre_layout(&mut self, ctx: LayoutCtx) -> LayoutHints {
        (**self).pre_layout(ctx)
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        (**self).layout(ctx, size)
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        (**self).draw(ctx, bounds)
    }

    fn handle(&mut self, event: Event, data: &mut D) {
        (**self).handle(event, data)
    }
}
