use std::collections::HashMap;

use crate::syntax::Ident;
use crate::vm::{RegId, UpfnId, UpvalueId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VarLoc {
    Reg(RegId),
    Upvalue(UpvalueId),
    PossibleUpvalue,
    Upfn(UpfnId),
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

#[derive(Clone, Debug, Default)]
pub struct Scope {
    vars: HashMap<Ident, VarLoc>,
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

    pub fn pop(&mut self) -> impl Iterator<Item = VarLoc> + '_ {
        let prev = self.stack.pop().unwrap();
        let next = self.scope();
        prev.vars
            .into_iter()
            .filter(|(name, loc)| next.vars.get(&name) != Some(loc))
            .map(|(_, loc)| loc)
    }

    pub fn get(&self, ident: &Ident) -> Option<VarLoc> {
        self.scope().vars.get(ident).copied()
    }

    pub fn set(&mut self, ident: Ident, loc: impl Into<VarLoc>) -> Option<VarLoc> {
        self.scope_mut().vars.insert(ident, loc.into())
    }

    pub fn names(&self) -> impl Iterator<Item = Ident> + '_ {
        self.scope().vars.keys().cloned()
    }
}
