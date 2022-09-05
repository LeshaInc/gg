use std::fmt::{self, Debug};
use std::ops::{Add, Sub};

use super::reg::{RegId, RegSeq};
pub use crate::syntax::{BinOp, UnOp};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Instr<Loc = RegId> {
    Nop,
    Panic,
    Copy {
        src: Loc,
        dst: Loc,
    },
    NewList {
        seq: RegSeq,
        dst: Loc,
    },
    NewMap {
        seq: RegSeq,
        dst: Loc,
    },
    Jump {
        offset: InstrOffset,
    },
    JumpIfTrue {
        cond: Loc,
        offset: InstrOffset,
    },
    JumpIfFalse {
        cond: Loc,
        offset: InstrOffset,
    },
    BinOp {
        op: BinOp,
        lhs: Loc,
        rhs: Loc,
        dst: Loc,
    },
    UnOp {
        op: UnOp,
        arg: Loc,
        dst: Loc,
    },
    Call {
        seq: RegSeq,
        dst: Loc,
    },
}

impl<Loc> Instr<Loc> {
    pub fn map_seq(self, mut f: impl FnMut(RegSeq) -> RegSeq) -> Instr<Loc> {
        match self {
            Instr::NewList { seq, dst } => Instr::NewList { seq: f(seq), dst },
            Instr::NewMap { seq, dst } => Instr::NewMap { seq: f(seq), dst },
            Instr::Call { seq, dst } => Instr::Call { seq: f(seq), dst },
            _ => self,
        }
    }

    pub fn map_loc<T>(self, mut f: impl FnMut(Loc) -> T) -> Instr<T> {
        match self {
            Instr::Nop => Instr::Nop,
            Instr::Panic => Instr::Panic,
            Instr::Copy { src, dst } => Instr::Copy {
                src: f(src),
                dst: f(dst),
            },
            Instr::NewList { seq, dst } => Instr::NewList { seq, dst: f(dst) },
            Instr::NewMap { seq, dst } => Instr::NewMap { seq, dst: f(dst) },
            Instr::Jump { offset } => Instr::Jump { offset },
            Instr::JumpIfTrue { cond, offset } => Instr::JumpIfTrue {
                cond: f(cond),
                offset,
            },
            Instr::JumpIfFalse { cond, offset } => Instr::JumpIfFalse {
                cond: f(cond),
                offset,
            },
            Instr::BinOp { op, lhs, rhs, dst } => Instr::BinOp {
                op,
                lhs: f(lhs),
                rhs: f(rhs),
                dst: f(dst),
            },
            Instr::UnOp { op, arg, dst } => Instr::UnOp {
                op,
                arg: f(arg),
                dst: f(dst),
            },
            Instr::Call { seq, dst } => Instr::Call { seq, dst: f(dst) },
        }
    }
}

impl<Loc: Debug> Debug for Instr<Loc> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instr::Nop => write!(f, "Nop"),
            Instr::Panic => write!(f, "Panic"),
            Instr::Copy { src, dst } => write!(f, "Copy\t{:?} -> {:?}", src, dst),
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
pub struct Instrs<Loc>(pub Vec<Instr<Loc>>);

impl<Loc> Instrs<Loc> {
    pub fn next_idx(&self) -> InstrIdx {
        InstrIdx(self.0.len() as u32)
    }

    pub fn add(&mut self, instr: Instr<Loc>) -> InstrIdx {
        let idx = self.next_idx();
        self.0.push(instr);
        idx
    }

    pub fn set(&mut self, idx: InstrIdx, instr: Instr<Loc>) {
        self.0[idx.0 as usize] = instr;
    }

    pub fn compile(
        self,
        mut loc_mapping: impl FnMut(Loc) -> RegId,
        mut seq_mapping: impl FnMut(RegSeq) -> RegSeq,
    ) -> CompiledInstrs {
        CompiledInstrs(
            self.0
                .into_iter()
                .map(|v| v.map_loc(&mut loc_mapping).map_seq(&mut seq_mapping))
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CompiledInstrs(pub Box<[Instr]>);
