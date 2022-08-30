use std::ops::{Add, Sub};

use super::consts::ConstId;
use super::reg::RegId;
use crate::syntax::{BinOp, UnOp};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstrIdx(pub u16);

impl Add<InstrOffset> for InstrIdx {
    type Output = InstrIdx;

    fn add(self, other: InstrOffset) -> InstrIdx {
        InstrIdx((self.0 as i16).wrapping_add(other.0) as u16)
    }
}

impl Sub<InstrIdx> for InstrIdx {
    type Output = InstrOffset;

    fn sub(self, other: InstrIdx) -> InstrOffset {
        InstrOffset((self.0 as i16).wrapping_sub(other.0 as i16))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstrOffset(pub i16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instr {
    Nop,
    LoadConst {
        id: ConstId,
        res: RegId,
    },
    BinOp {
        op: BinOp,
        lhs: RegId,
        rhs: RegId,
        res: RegId,
    },
    UnOp {
        op: UnOp,
        val: BinOp,
        res: RegId,
    },
}

#[derive(Clone, Debug, Default)]
pub struct Instrs(pub Vec<Instr>);

impl Instrs {
    pub fn next_idx(&self) -> InstrIdx {
        InstrIdx(self.0.len() as u16)
    }

    pub fn add(&mut self, instr: Instr) -> InstrIdx {
        let idx = self.next_idx();
        self.0.push(instr);
        idx
    }
}
