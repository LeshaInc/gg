use crate::Map;

pub mod math;

pub fn builtins() -> Map {
    let mut map = Map::new();
    map.insert("math".into(), math::module());
    map
}
