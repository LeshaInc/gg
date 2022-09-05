use crate::vm::{ConstId, RegId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Loc {
    Invalid,
    Tmp(RegId),
    Persistent(RegId),
    Const(ConstId),
}

impl Default for Loc {
    fn default() -> Loc {
        Loc::Invalid
    }
}
