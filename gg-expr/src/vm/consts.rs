use std::collections::HashMap;
use std::fmt::{self, Debug};

use crate::Value;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstId(pub u16);

impl Debug for ConstId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c{}", self.0)
    }
}

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

    pub fn len(&self) -> u16 {
        self.0.len() as u16
    }

    pub fn compile(self) -> CompiledConsts {
        let mut pairs = self.0.into_iter().collect::<Vec<_>>();
        pairs.sort_unstable_by_key(|(_, id)| *id);
        let values = pairs.into_iter().map(|(v, _)| v).collect();
        CompiledConsts(values)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CompiledConsts(pub Box<[Value]>);
