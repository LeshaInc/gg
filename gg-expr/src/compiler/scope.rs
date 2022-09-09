use std::collections::HashMap;

use crate::syntax::Ident;
use crate::vm::RegId;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct CaptureId(pub u16);

#[derive(Clone, Debug, Default)]
pub struct Scope {
    vars: HashMap<Ident, RegId>,
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

    pub fn pop(&mut self) -> impl Iterator<Item = RegId> + '_ {
        let prev = self.stack.pop().unwrap();
        let next = self.scope();
        prev.vars
            .into_iter()
            .filter(|(name, loc)| next.vars.get(&name) != Some(loc))
            .map(|(_, loc)| loc)
    }

    pub fn get(&self, ident: &Ident) -> Option<RegId> {
        self.scope().vars.get(ident).copied()
    }

    pub fn set(&mut self, ident: Ident, loc: impl Into<RegId>) -> Option<RegId> {
        self.scope_mut().vars.insert(ident, loc.into())
    }

    pub fn names(&self) -> impl Iterator<Item = Ident> + '_ {
        self.scope().vars.keys().cloned()
    }
}
