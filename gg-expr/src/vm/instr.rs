use std::fmt::{self, Debug};
use std::ops::{Add, AddAssign, Index, Sub};
use std::sync::Arc;

use super::reg::{RegId, RegSeq};
use super::{ConstId, UpfnId, UpvalueId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Opcode {
    Nop,
    Panic,
    LoadConst,
    LoadUpvalue,
    LoadUpfn,
    Copy,
    CopyIfTrue,
    NewList,
    NewMap,
    NewFunc,
    Jump,
    JumpIfTrue,
    JumpIfFalse,
    Call,
    TailCall,
    Ret,

    OpOr,
    OpCoalesce,
    OpAnd,
    OpLt,
    OpLe,
    OpEq,
    OpNeq,
    OpGe,
    OpGt,
    OpAdd,
    OpSub,
    OpMul,
    OpDiv,
    OpRem,
    OpPow,
    OpIndex,
    OpIndexNullable,

    UnOpNeg,
    UnOpNot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Operand {
    None,
    ConstId,
    UpvalueId,
    RegA,
    RegB,
    RegC,
    RegSeq,
    Offset,
}

impl Opcode {
    pub fn operator(self) -> &'static str {
        use Opcode::*;

        match self {
            OpOr => "||",
            OpCoalesce => "??",
            OpAnd => "&&",
            OpLt => "<",
            OpLe => "<=",
            OpEq => "==",
            OpNeq => "!=",
            OpGe => ">=",
            OpGt => ">",
            OpAdd => "+",
            OpSub => "-",
            OpMul => "*",
            OpDiv => "/",
            OpRem => "%",
            OpPow => "**",
            OpIndex => "[]",
            OpIndexNullable => "?[]",
            UnOpNeg => "-",
            UnOpNot => "!",
            _ => "?",
        }
    }

    pub fn operands(self) -> [Operand; 3] {
        use Opcode::*;
        use Operand::*;
        match self {
            Nop | Panic => [None; 3],
            LoadConst => [ConstId, RegB, None],
            LoadUpvalue => [UpvalueId, RegB, None],
            LoadUpfn => [UpvalueId, RegB, None],
            Copy => [RegA, RegB, None],
            CopyIfTrue => [RegA, RegB, RegC],
            NewList | NewMap | NewFunc => [RegSeq, RegC, None],
            Jump => [Offset, None, None],
            JumpIfTrue | JumpIfFalse => [RegA, Offset, None],
            Call => [RegSeq, RegC, None],
            TailCall => [RegSeq, RegC, None],
            Ret => [RegA, None, None],
            OpOr | OpCoalesce | OpAnd | OpLt | OpLe | OpEq | OpNeq | OpGe | OpGt | OpAdd
            | OpSub | OpMul | OpDiv | OpRem | OpPow | OpIndex | OpIndexNullable => {
                [RegA, RegB, RegC]
            }
            UnOpNeg | UnOpNot => [RegA, RegB, None],
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Instr {
    pub opcode: Opcode,
    pub operands: [u16; 3],
}

impl Instr {
    pub fn new(opcode: Opcode) -> Self {
        Self {
            opcode,
            operands: [0; 3],
        }
    }

    pub fn const_id(self) -> ConstId {
        ConstId(self.operands[0])
    }

    pub fn with_const_id(mut self, id: ConstId) -> Self {
        self.operands[0] = id.0;
        self
    }

    pub fn upvalue_id(self) -> UpvalueId {
        UpvalueId(self.operands[0])
    }

    pub fn with_upvalue_id(mut self, id: UpvalueId) -> Self {
        self.operands[0] = id.0;
        self
    }

    pub fn upfn_id(self) -> UpfnId {
        UpfnId(self.operands[0])
    }

    pub fn with_upfn_id(mut self, id: UpfnId) -> Self {
        self.operands[0] = id.0;
        self
    }

    pub fn offset(self) -> InstrOffset {
        let hi = self.operands[1].to_le_bytes();
        let lo = self.operands[2].to_le_bytes();
        InstrOffset(i32::from_le_bytes([hi[0], hi[1], lo[0], lo[1]]))
    }

    pub fn with_offset(mut self, offset: InstrOffset) -> Self {
        let ofs = offset.0.to_le_bytes();
        self.operands[1] = u16::from_le_bytes([ofs[0], ofs[1]]);
        self.operands[2] = u16::from_le_bytes([ofs[2], ofs[3]]);
        self
    }

    pub fn reg_seq(self) -> RegSeq {
        RegSeq {
            base: RegId(self.operands[0]),
            len: self.operands[1],
        }
    }

    pub fn with_reg_seq(mut self, seq: RegSeq) -> Self {
        self.operands[0] = seq.base.0;
        self.operands[1] = seq.len;
        self
    }

    pub fn reg_a(self) -> RegId {
        RegId(self.operands[0])
    }

    pub fn with_reg_a(mut self, reg: RegId) -> Self {
        self.operands[0] = reg.0;
        self
    }

    pub fn reg_b(self) -> RegId {
        RegId(self.operands[1])
    }

    pub fn with_reg_b(mut self, reg: RegId) -> Self {
        self.operands[1] = reg.0;
        self
    }

    pub fn reg_c(self) -> RegId {
        RegId(self.operands[2])
    }

    pub fn with_reg_c(mut self, reg: RegId) -> Self {
        self.operands[2] = reg.0;
        self
    }
}

impl Debug for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:11}", format!("{:?}", self.opcode))?;

        let operands = self.opcode.operands();
        if operands[0] == Operand::None {
            return Ok(());
        };

        write!(f, " ")?;

        for (i, operand) in operands.into_iter().enumerate() {
            if operand == Operand::None {
                break;
            }
            if i > 0 {
                write!(f, ", ")?;
            }

            match operand {
                Operand::ConstId => self.const_id().fmt(f)?,
                Operand::UpvalueId => self.upvalue_id().fmt(f)?,
                Operand::RegA => self.reg_a().fmt(f)?,
                Operand::RegB => self.reg_b().fmt(f)?,
                Operand::RegC => self.reg_c().fmt(f)?,
                Operand::RegSeq => self.reg_seq().fmt(f)?,
                Operand::Offset => self.offset().fmt(f)?,
                Operand::None => {}
            }
        }

        Ok(())
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

impl AddAssign<InstrOffset> for InstrIdx {
    fn add_assign(&mut self, rhs: InstrOffset) {
        *self = *self + rhs;
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

    pub fn last_idx(&self) -> InstrIdx {
        InstrIdx(self.0.len() as u32 - 1)
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
pub struct CompiledInstrs(pub Arc<[Instr]>);

impl Index<InstrIdx> for CompiledInstrs {
    type Output = Instr;

    fn index(&self, index: InstrIdx) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}
