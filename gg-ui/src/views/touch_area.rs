use std::marker::PhantomData;

use gg_math::Rect;

use crate::{Event, HandleCtx, LayoutCtx, LayoutHints, UiAction, View};

pub fn touch_area<D, F>(callback: F) -> TouchArea<D, F>
where
    F: FnMut(&mut D),
{
    TouchArea {
        phantom: PhantomData,
        callback,
    }
}

pub struct TouchArea<D, F> {
    phantom: PhantomData<fn(D)>,
    callback: F,
}

impl<D, F> View<D> for TouchArea<D, F>
where
    F: FnMut(&mut D),
{
    fn pre_layout(&mut self, _ctx: LayoutCtx) -> LayoutHints {
        LayoutHints {
            stretch: 1.0,
            ..LayoutHints::default()
        }
    }

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event) {
        if event.pressed_action(UiAction::Touch) && bounds.contains(ctx.input.mouse_pos()) {
            (self.callback)(ctx.data);
        }
    }
}
