use crate::vm::{ConstId, RegId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Loc {
    Invalid,
    Reg(RegId),
    Const(ConstId),
}

impl From<RegId> for Loc {
    fn from(v: RegId) -> Loc {
        Loc::Reg(v)
    }
}

impl From<ConstId> for Loc {
    fn from(v: ConstId) -> Loc {
        Loc::Const(v)
    }
}

impl Default for Loc {
    fn default() -> Loc {
        Loc::Invalid
    }
}
