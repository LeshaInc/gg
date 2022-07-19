use crossbeam_channel::{Receiver, Sender};
use gg_util::rtti::TypeId;

use crate::event::EventKind;
use crate::id::UntypedId;
use crate::storage::AnyAsset;
use crate::{Asset, Assets, Id};

pub enum Command {
    Insert(UntypedId, TypeId, Box<dyn AnyAsset>),
    Remove(UntypedId, TypeId),
    Closure(Box<dyn FnOnce(&mut Assets) + Send + Sync>),
}

impl Command {
    pub fn execute(self, assets: &mut Assets) {
        match self {
            Command::Insert(id, ty, value) => {
                let event_kind = if assets.storage.contains_untyped(id, ty) {
                    EventKind::Updated
                } else {
                    EventKind::Created
                };

                assets.storage.insert_any(id, ty, value);
                assets.shared.send_event_untyped(event_kind, id, ty);

                let meta_storage = assets.shared.metadata.read();
                if let Some(meta) = meta_storage.get(id) {
                    meta.available.set(true);
                }
            }

            Command::Remove(id, ty) => {
                assets.storage.remove(id, ty);
                assets.shared.metadata.write().remove(id);
                assets.shared.send_event_untyped(EventKind::Removed, id, ty);
            }

            Command::Closure(closure) => {
                closure(assets);
            }
        }
    }
}

pub fn new_command_channel() -> (CommandSender, CommandReceiver) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    (CommandSender { sender }, CommandReceiver { receiver })
}

#[derive(Debug, Clone)]
pub struct CommandSender {
    sender: Sender<Command>,
}

impl CommandSender {
    pub fn send(&self, command: Command) {
        let _ = self.sender.send(command);
    }

    pub fn insert<T: Asset>(&self, id: Id<T>, asset: T) {
        self.insert_untyped(id.into_untyped(), TypeId::of::<T>(), Box::new(asset));
    }

    pub fn insert_untyped(&self, id: UntypedId, ty: TypeId, asset: Box<dyn AnyAsset>) {
        self.send(Command::Insert(id, ty, asset));
    }

    pub fn remove_untyped(&self, id: UntypedId, ty: TypeId) {
        self.send(Command::Remove(id, ty));
    }

    pub fn closure<F>(&self, command: F)
    where
        F: FnOnce(&mut Assets) + Send + Sync + 'static,
    {
        self.send(Command::Closure(Box::new(command)));
    }
}

#[derive(Debug)]
pub struct CommandReceiver {
    receiver: Receiver<Command>,
}

impl CommandReceiver {
    pub fn try_recv(&self) -> Option<Command> {
        self.receiver.try_recv().ok()
    }
}
