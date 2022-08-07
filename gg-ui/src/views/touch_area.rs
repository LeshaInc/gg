use gg_math::Rect;

use crate::{Event, HandleCtx, LayoutCtx, LayoutHints, UiAction, View};

pub fn touch_area<D, F>(callback: F) -> TouchArea<F>
where
    F: FnOnce(&mut D),
{
    TouchArea {
        callback: Some(callback),
    }
}

pub struct TouchArea<F> {
    callback: Option<F>,
}

impl<D, F> View<D> for TouchArea<F>
where
    F: FnOnce(&mut D),
{
    fn pre_layout(&mut self, _ctx: LayoutCtx) -> LayoutHints {
        LayoutHints {
            stretch: 1.0,
            ..LayoutHints::default()
        }
    }

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event) {
        if event.pressed_action(UiAction::Touch) && bounds.contains(ctx.input.mouse_pos()) {
            if let Some(callback) = self.callback.take() {
                callback(ctx.data);
            }
        }
    }
}
