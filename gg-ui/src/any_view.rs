use std::any::Any;

use gg_math::Vec2;

use crate::{Bounds, DrawCtx, Event, Hover, LayoutCtx, LayoutHints, UpdateCtx, View};

pub trait AnyView<D: 'static>: Any + View<D> {
    fn as_any(&mut self) -> &mut dyn Any;

    fn init_dyn(&mut self, old: &mut dyn AnyView<D>) -> bool;
}

impl<D: 'static, V: Any + View<D>> AnyView<D> for V {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn init_dyn(&mut self, old: &mut dyn AnyView<D>) -> bool {
        if let Some(old) = old.as_any().downcast_mut::<Self>() {
            self.init(old)
        } else {
            true
        }
    }
}

impl<'a, D: 'static> View<D> for Box<dyn AnyView<D>> {
    fn init(&mut self, old: &mut Self) -> bool {
        (**self).init_dyn(&mut **old)
    }

    fn pre_layout(&mut self, ctx: &mut LayoutCtx) -> LayoutHints {
        (**self).pre_layout(ctx)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        (**self).layout(ctx, size)
    }

    fn hover(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) -> Hover {
        (**self).hover(ctx, bounds)
    }

    fn update(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds) {
        (**self).update(ctx, bounds)
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) {
        (**self).handle(ctx, bounds, event)
    }

    fn draw(&mut self, ctx: &mut DrawCtx, bounds: Bounds) {
        (**self).draw(ctx, bounds)
    }
}
