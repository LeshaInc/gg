use gg_util::ahash::AHashMap;
use gg_util::rtti::TypeId;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Action {
    type_id: TypeId,
    ordinal: u32,
}

impl Action {
    pub fn new<A: ActionKind>(kind: A) -> Action {
        kind.into()
    }

    pub fn cast<A: ActionKind>(self) -> Option<A> {
        if self.type_id != TypeId::of::<A>() {
            return None;
        }

        A::from_ordinal(self.ordinal)
    }
}

pub trait ActionKind: Sized + Copy + Eq + 'static {
    const ACTIONS: &'static [&'static str];

    fn ordinal(self) -> u32;

    fn from_ordinal(v: u32) -> Option<Self>;
}

impl<A: ActionKind> From<A> for Action {
    fn from(action: A) -> Self {
        Action {
            type_id: TypeId::of::<A>(),
            ordinal: action.ordinal(),
        }
    }
}

#[macro_export]
macro_rules! action {
    (pub enum $enum:ident { $($variant:ident = $name:literal,)+ }) => {
        #[repr(u32)]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub enum $enum {
            $($variant,)+
        }

        impl gg_input::ActionKind for $enum {
            const ACTIONS: &'static [&'static str] = &[
                $($name,)+
            ];

            fn ordinal(self) -> u32 {
                self as u32
            }

            fn from_ordinal(v: u32) -> Option<Self> {
                if v < $crate::action!(@count $($variant,)+) {
                    Some(unsafe { std::mem::transmute(v) })
                } else {
                    None
                }
            }
        }
    };

    (@count $head:ident,) => {
        1
    };

    (@count $head:ident, $($tail:ident,)+) => {
        action!(@count $($tail,)+) + 1
    };
}

#[derive(Debug, Default, Clone)]
pub struct ActionRegistry {
    map: AHashMap<&'static str, Action>,
}

impl ActionRegistry {
    pub fn register<A: ActionKind>(&mut self) {
        let type_id = TypeId::of::<A>();

        for (ordinal, name) in A::ACTIONS.iter().enumerate() {
            if self.map.contains_key(name) {
                panic!("duplicate definitions for action `{}`", name);
            }

            let action = Action {
                type_id,
                ordinal: ordinal as u32,
            };

            self.map.insert(name, action);
        }
    }

    pub fn get(&self, name: &str) -> Option<Action> {
        self.map.get(name).copied()
    }
}
