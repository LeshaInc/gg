use std::collections::HashMap;

use crate::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstId(pub u16);

#[derive(Clone, Debug, Default)]
pub struct Consts(pub HashMap<Value, ConstId>);

impl Consts {
    pub fn add(&mut self, value: Value) -> ConstId {
        if let Some(&idx) = self.0.get(&value) {
            return idx;
        }

        let idx = ConstId(self.0.len() as u16);
        self.0.insert(value, idx);
        idx
    }
}
