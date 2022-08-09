use crate::{views, SetChildren, View, ViewExt};

pub fn button<D>(
    label: impl Into<String>,
    callback: impl FnOnce(&mut D) + 'static,
) -> impl View<D> {
    views::stateful(0, |state| {
        let label = format!("{} ({})", label.into(), state);

        views::overlay().padding(4.0).children((
            views::rect([0.1; 3]),
            views::text(label).wrap(false).padding([10.0, 20.0]),
            views::touch_area(|(data, state)| {
                *state += 1;
                callback(data)
            }),
            views::nothing().stretch(2.0),
        ))
    })
}
