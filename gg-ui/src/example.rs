#![allow(dead_code)]

use crate::views::{button, choose};
use crate::View;

pub struct Data {
    pub count: i32,
}

pub fn my_ui(data: &Data) -> impl View<Data> {
    choose(
        data.count <= 10,
        button("Hello!", |d: &mut Data| d.count += 1),
        button("I hate you!", |d: &mut Data| d.count += 1),
    )
}
