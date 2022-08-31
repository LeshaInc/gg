use std::ops::{Add, Sub};

use super::consts::ConstId;
use super::reg::{RegId, RegSeq};
use crate::syntax::{BinOp, UnOp};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstrOffset(pub i32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instr {
    Nop,
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
}
