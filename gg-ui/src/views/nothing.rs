use std::marker::PhantomData;

use crate::View;

pub fn nothing<D>() -> Nothing<D> {
    Nothing {
        phantom: PhantomData,
    }
}

impl<D> View<D> for Nothing<D> {}

pub struct Nothing<D> {
    phantom: PhantomData<fn(&mut D)>,
}
