use std::fmt::{self, Debug};
use std::hash::{self, Hash};
use std::marker::PhantomData;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct UntypedId(pub(crate) u64);

impl Debug for UntypedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Id<T> {
    untyped: UntypedId,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub(crate) fn from_untyped(untyped: UntypedId) -> Id<T> {
        Id {
            untyped,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn into_untyped(self) -> UntypedId {
        self.untyped
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", std::any::type_name::<T>(), self.untyped.0)
    }
}

impl<T> Eq for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Id<T>) -> bool {
        self.untyped == other.untyped
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.untyped.hash(state)
    }
}
