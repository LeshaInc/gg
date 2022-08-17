use std::fmt::{self, Debug, Write};

use indenter::indented;

use crate::syntax::{BinOp, UnOp};
use crate::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstId(pub u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StackPos(pub u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstrOffset(pub i16);

#[derive(Clone, Copy, Debug)]
pub enum Instr {
    Nop,
    PushCopy(StackPos),
    PushConst(ConstId),
    Pop,
    Jump(InstrOffset),
    JumpIf(InstrOffset),
    UnOp(UnOp),
    BinOp(BinOp),
}

#[derive(Clone)]
pub struct Func {
    pub arity: usize,
    pub instrs: Vec<Instr>,
    pub consts: Vec<Value>,
}

impl Func {
    pub fn new(arity: usize) -> Func {
        Func {
            arity,
            instrs: Vec::new(),
            consts: Vec::new(),
        }
    }

    pub fn add_const(&mut self, val: Value) -> ConstId {
        let id = u16::try_from(self.consts.len()).expect("too many constants");
        self.consts.push(val);
        ConstId(id)
    }

    pub fn add_instr(&mut self, instr: Instr) -> usize {
        let idx = self.instrs.len();
        self.instrs.push(instr);
        idx
    }
}

impl Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fn({}):", self.arity)?;

        if !self.consts.is_empty() {
            let mut f = indented(f);
            writeln!(f, "consts:")?;

            for (i, val) in self.consts.iter().enumerate() {
                writeln!(indented(&mut f), "{}: {:?}", i, val)?;
            }
        }

        for (i, instr) in self.instrs.iter().enumerate() {
            let mut f = indented(f);

            if i > 0 {
                writeln!(f)?;
            }

            write!(f, "{:?}", instr)?;
        }

        Ok(())
    }
}
