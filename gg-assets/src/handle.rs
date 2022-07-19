use std::fmt::{self, Debug};
use std::hash::{self, Hash};
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

use gg_util::rtti::{type_name_of_id, TypeId};

use crate::command::CommandSender;
use crate::id::{Id, UntypedId};

pub struct Handle<T> {
    untyped: UntypedHandle,
    _phantom: PhantomData<fn() -> T>,
}

impl<T: 'static> Handle<T> {
    pub(crate) fn from_untyped(untyped: UntypedHandle) -> Handle<T> {
        Handle {
            untyped,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn as_untyped(&self) -> &UntypedHandle {
        &self.untyped
    }

    pub fn id(&self) -> Id<T> {
        Id::from_untyped(self.untyped.id())
    }

    pub fn downgrade(&self) -> WeakHandle<T> {
        WeakHandle::from_untyped(self.untyped.downgrade())
    }
}

impl<T: 'static> Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id().fmt(f)
    }
}

impl<T: 'static> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Handle {
            untyped: self.untyped.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static> Eq for Handle<T> {}

impl<T: 'static> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.untyped == other.untyped
    }
}

pub struct WeakHandle<T> {
    untyped: UntypedWeakHandle,
    _phantom: PhantomData<T>,
}

impl<T: 'static> WeakHandle<T> {
    pub(crate) fn from_untyped(untyped: UntypedWeakHandle) -> WeakHandle<T> {
        WeakHandle {
            untyped,
            _phantom: PhantomData,
        }
    }

    pub fn upgrade(&self) -> Option<Handle<T>> {
        self.untyped.upgrade().map(Handle::from_untyped)
    }
}

impl<T: 'static> Debug for WeakHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(weak)")
    }
}

impl<T: 'static> Clone for WeakHandle<T> {
    fn clone(&self) -> Self {
        WeakHandle {
            untyped: self.untyped.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static> Eq for WeakHandle<T> {}

impl<T: 'static> PartialEq for WeakHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.untyped == other.untyped
    }
}

#[derive(Clone)]
pub struct UntypedHandle {
    shared: Arc<HandleShared>,
}

impl UntypedHandle {
    pub fn new(id: UntypedId, ty: TypeId, command_sender: CommandSender) -> UntypedHandle {
        UntypedHandle {
            shared: Arc::new(HandleShared {
                id,
                ty,
                command_sender,
            }),
        }
    }

    pub fn id(&self) -> UntypedId {
        self.shared.id
    }

    pub fn ty(&self) -> TypeId {
        self.shared.ty
    }

    pub fn downgrade(&self) -> UntypedWeakHandle {
        UntypedWeakHandle {
            shared: Arc::downgrade(&self.shared),
        }
    }
}

impl Debug for UntypedHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(type_name) = type_name_of_id(self.ty()) {
            write!(f, "{}({})", type_name, self.id().0)
        } else {
            write!(f, "{}", self.id().0)
        }
    }
}

impl Eq for UntypedHandle {}

impl PartialEq for UntypedHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.shared, &other.shared)
    }
}

struct HandleShared {
    id: UntypedId,
    ty: TypeId,
    command_sender: CommandSender,
}

impl Drop for HandleShared {
    fn drop(&mut self) {
        self.command_sender.remove_untyped(self.id, self.ty);
    }
}

#[derive(Clone)]
pub struct UntypedWeakHandle {
    shared: Weak<HandleShared>,
}

impl UntypedWeakHandle {
    pub fn upgrade(&self) -> Option<UntypedHandle> {
        Some(UntypedHandle {
            shared: self.shared.upgrade()?,
        })
    }
}

impl Debug for UntypedWeakHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(weak)")
    }
}

impl Eq for UntypedWeakHandle {}

impl PartialEq for UntypedWeakHandle {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.shared, &other.shared)
    }
}

impl Hash for UntypedWeakHandle {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.shared.as_ptr().hash(state)
    }
}
