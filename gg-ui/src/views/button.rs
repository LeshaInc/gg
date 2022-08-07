use crate::{views, View, ViewExt};

pub fn button<D>(label: impl Into<String>, mut callback: impl FnMut(&mut D)) -> impl View<D> {
    views::overlay((
        views::rect([0.1; 3]),
        views::text(label).min_width(100.0).padding([10.0, 20.0]),
        views::touch_area(move |data| (callback)(data)),
    ))
    .padding(8.0)
}
