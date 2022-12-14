use crate::{Bounds, Event, LayoutCtx, LayoutHints, UiAction, UpdateCtx, View};

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
    fn pre_layout(&mut self, _ctx: &mut LayoutCtx) -> LayoutHints {
        LayoutHints {
            stretch: 1.0,
            ..LayoutHints::default()
        }
    }

    fn handle(&mut self, ctx: &mut UpdateCtx<D>, bounds: Bounds, event: Event) -> bool {
        if event.pressed_action(UiAction::Touch) && bounds.hover.is_direct() {
            if let Some(callback) = self.callback.take() {
                callback(ctx.data);
                return true;
            }
        }

        false
    }
}
