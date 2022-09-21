use std::collections::HashMap;

use crate::syntax::Ident;
use crate::vm::{RegId, UpfnId, UpvalueId};
use crate::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VarLoc {
    Reg(RegId),
    Upvalue(UpvalueId),
    PossibleUpvalue,
    Upfn(UpfnId),
    Value(Value),
}

impl From<RegId> for VarLoc {
    fn from(v: RegId) -> VarLoc {
        VarLoc::Reg(v)
    }
}

impl From<UpvalueId> for VarLoc {
    fn from(v: UpvalueId) -> VarLoc {
        VarLoc::Upvalue(v)
    }
}

impl From<UpfnId> for VarLoc {
    fn from(v: UpfnId) -> VarLoc {
        VarLoc::Upfn(v)
    }
}

impl From<Value> for VarLoc {
    fn from(v: Value) -> VarLoc {
        VarLoc::Value(v)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Scope {
    vars: HashMap<Ident, VarLoc>,
    locs: Vec<VarLoc>,
}

#[derive(Clone, Debug)]
pub struct ScopeStack {
    stack: Vec<Scope>,
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self {
            stack: vec![Scope::default()],
        }
    }
}

impl ScopeStack {
    pub fn scope(&self) -> &Scope {
        self.stack.last().unwrap()
    }

    pub fn scope_mut(&mut self) -> &mut Scope {
        self.stack.last_mut().unwrap()
    }

    pub fn push(&mut self) {
        let scope = self.scope().clone();
        self.stack.push(scope);
    }

    pub fn pop(&mut self) -> impl Iterator<Item = VarLoc> {
        let prev = self.stack.pop().unwrap();
        prev.locs.into_iter()
    }

    pub fn get(&self, ident: &Ident) -> Option<&VarLoc> {
        self.scope().vars.get(ident)
    }

    pub fn set(&mut self, ident: Ident, loc: impl Into<VarLoc>) {
        let loc = loc.into();
        let scope = self.scope_mut();
        scope.vars.insert(ident, loc.clone());
        scope.locs.push(loc);
    }

    pub fn names(&self) -> impl Iterator<Item = Ident> + '_ {
        self.scope().vars.keys().cloned()
    }
}
