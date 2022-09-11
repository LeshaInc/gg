use std::fmt::{self, Debug};
use std::ops::Index;

use crate::syntax::Ident;
use crate::Value;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UpvalueId(pub u16);

impl Debug for UpvalueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "u{}", self.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct UpvalueNames(pub Vec<Ident>);

impl UpvalueNames {
    pub fn add(&mut self, name: Ident) -> UpvalueId {
        let idx = UpvalueId(self.0.len() as u16);
        self.0.push(name);
        idx
    }

    pub fn len(&self) -> u16 {
        self.0.len() as u16
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &Ident> + '_ {
        self.0.iter()
    }

    pub fn compile(self) -> Upvalues {
        Upvalues(vec![Value::null(); self.0.len()].into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Upvalues(pub Box<[Value]>);

impl Upvalues {
    pub fn get(&self, id: UpvalueId) -> Option<&Value> {
        self.0.get(id.0 as usize)
    }
}

impl Index<UpvalueId> for Upvalues {
    type Output = Value;

    fn index(&self, idx: UpvalueId) -> &Value {
        self.get(idx).unwrap()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UpfnId(pub u16);

impl Debug for UpfnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "uf{}", self.0)
    }
}
