use crate::{LayoutCtx, LayoutHints, View};

pub fn nothing() -> Nothing {
    Nothing
}

impl<D> View<D> for Nothing {
    fn pre_layout(&mut self, _: LayoutCtx) -> LayoutHints {
        LayoutHints {
            stretch: 0.0,
            ..Default::default()
        }
    }
}

pub struct Nothing;
