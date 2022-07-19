use std::sync::atomic::{AtomicU64, Ordering};

use gg_util::rtti::TypeId;

use crate::command::CommandSender;
use crate::handle::UntypedHandle;
use crate::id::UntypedId;
use crate::Handle;

#[derive(Debug)]
pub struct HandleAllocator {
    id_counter: AtomicU64,
    command_sender: CommandSender,
}

impl HandleAllocator {
    pub fn new(command_sender: CommandSender) -> HandleAllocator {
        HandleAllocator {
            id_counter: AtomicU64::new(0),
            command_sender,
        }
    }

    pub fn alloc<T: 'static>(&self) -> Handle<T> {
        Handle::from_untyped(self.alloc_untyped(TypeId::of::<T>()))
    }

    pub fn alloc_untyped(&self, ty: TypeId) -> UntypedHandle {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        assert!(id < u64::max_value(), "asset id overflow");
        UntypedHandle::new(UntypedId(id), ty, self.command_sender.clone())
    }
}
