use std::fmt::{self, Debug};
use std::ops::{Add, Sub};

use super::consts::ConstId;
use super::reg::{RegId, RegSeq};
pub use crate::syntax::{BinOp, UnOp};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Instr {
    Nop,
    Panic,
    Copy {
        src: RegId,
        dst: RegId,
    },
    LoadConst {
        id: ConstId,
        dst: RegId,
    },
    NewList {
        seq: RegSeq,
        dst: RegId,
    },
    NewMap {
        seq: RegSeq,
        dst: RegId,
    },
    Jump {
        offset: InstrOffset,
    },
    JumpIfTrue {
        cond: RegId,
        offset: InstrOffset,
    },
    JumpIfFalse {
        cond: RegId,
        offset: InstrOffset,
    },
    BinOp {
        op: BinOp,
        lhs: RegId,
        rhs: RegId,
        dst: RegId,
    },
    UnOp {
        op: UnOp,
        arg: RegId,
        dst: RegId,
    },
    Call {
        seq: RegSeq,
        dst: RegId,
    },
}

impl Debug for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::Nop => write!(f, "Nop"),
            Instr::Panic => write!(f, "Panic"),
            Instr::Copy { src, dst } => write!(f, "Copy\t{:?} -> {:?}", src, dst),
            Instr::LoadConst { id, dst } => write!(f, "LoadConst\t{:?} -> {:?}", id, dst),
            Instr::NewList { seq, dst } => write!(f, "NewList\t{:?} -> {:?}", seq, dst),
            Instr::NewMap { seq, dst } => write!(f, "NewMap\t{:?} -> {:?}", seq, dst),
            Instr::Jump { offset } => write!(f, "Jump\t{:?}", offset),
            Instr::JumpIfTrue { cond, offset } => write!(f, "JumpIfTrue\t{:?}, {:?}", cond, offset),
            Instr::JumpIfFalse { cond, offset } => {
                write!(f, "JumpIfFalse\t{:?}, {:?}", cond, offset)
            }
            Instr::BinOp { op, lhs, rhs, dst } => {
                write!(f, "BinOp\t{:?}({:?}, {:?}) -> {:?}", op, lhs, rhs, dst)
            }
            Instr::UnOp { op, arg, dst } => write!(f, "UnOp\t{:?}({:?}) -> {:?}", op, arg, dst),
            Instr::Call { seq, dst } => write!(f, "Call\t{:?} -> {:?}", seq, dst),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InstrIdx(pub u32);

impl Add<InstrOffset> for InstrIdx {
    type Output = InstrIdx;

    fn add(self, other: InstrOffset) -> InstrIdx {
        InstrIdx((self.0 as i32).wrapping_add(other.0) as u32)
    }
}

impl Sub<InstrIdx> for InstrIdx {
    type Output = InstrOffset;

    fn sub(self, other: InstrIdx) -> InstrOffset {
        InstrOffset((self.0 as i32).wrapping_sub(other.0 as i32))
    }
}

impl Add<i32> for InstrOffset {
    type Output = InstrOffset;

    fn add(self, other: i32) -> InstrOffset {
        InstrOffset(self.0 + other)
    }
}

impl Sub<i32> for InstrOffset {
    type Output = InstrOffset;

    fn sub(self, other: i32) -> InstrOffset {
        InstrOffset(self.0 - other)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct InstrOffset(pub i32);

impl Debug for InstrOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IP{:+}", self.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Instrs(pub Vec<Instr>);

impl Instrs {
    pub fn next_idx(&self) -> InstrIdx {
        InstrIdx(self.0.len() as u32)
    }

    pub fn add(&mut self, instr: Instr) -> InstrIdx {
        let idx = self.next_idx();
        self.0.push(instr);
        idx
    }

    pub fn set(&mut self, idx: InstrIdx, instr: Instr) {
        self.0[idx.0 as usize] = instr;
    }

    pub fn compile(self) -> CompiledInstrs {
        CompiledInstrs(self.0.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CompiledInstrs(pub Box<[Instr]>);
