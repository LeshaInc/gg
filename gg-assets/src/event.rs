use std::marker::PhantomData;

use crossbeam_channel::{Receiver, Sender};
use gg_util::ahash::AHashMap;
use gg_util::rtti::TypeId;

use crate::id::UntypedId;
use crate::{Asset, Id};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Event<A> {
    pub kind: EventKind,
    pub asset: Id<A>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum EventKind {
    Created,
    Updated,
    Removed,
}

#[derive(Debug)]
pub struct EventSender {
    sender: Sender<(EventKind, UntypedId)>,
}

impl EventSender {
    pub fn send(&self, kind: EventKind, id: UntypedId) {
        let _ = self.sender.send((kind, id));
    }
}

#[derive(Clone, Debug)]
pub struct EventReceiver<A> {
    receiver: Receiver<(EventKind, UntypedId)>,
    marker: PhantomData<Id<A>>,
}

impl<A: Asset> EventReceiver<A> {
    pub fn try_iter(&self) -> impl Iterator<Item = Event<A>> + '_ {
        self.receiver.try_iter().map(|(kind, id)| Event {
            kind,
            asset: Id::from_untyped(id),
        })
    }
}

pub fn create_event_channel<A>() -> (EventSender, EventReceiver<A>) {
    let (sender, receiver) = crossbeam_channel::unbounded();
    (
        EventSender { sender },
        EventReceiver {
            receiver,
            marker: PhantomData,
        },
    )
}

#[derive(Debug, Default)]
pub struct EventSenders {
    map: AHashMap<TypeId, Vec<EventSender>>,
}

impl EventSenders {
    pub fn new() -> EventSenders {
        EventSenders::default()
    }

    pub fn subscribe<A: Asset>(&mut self) -> EventReceiver<A> {
        let (sender, receiver) = create_event_channel();
        let senders = self.map.entry(TypeId::of::<A>()).or_default();
        senders.push(sender);
        receiver
    }

    pub fn send<A: Asset>(&self, kind: EventKind, id: Id<A>) {
        self.send_untyped(kind, id.into_untyped(), TypeId::of::<A>())
    }

    pub fn send_untyped(&self, kind: EventKind, id: UntypedId, ty: TypeId) {
        if let Some(senders) = self.map.get(&ty) {
            for sender in senders {
                sender.send(kind, id);
            }
        }
    }
}
