use std::fmt::{self, Debug, Write};

use indenter::indented;

use crate::syntax::{BinOp, UnOp};
use crate::Value;

#[derive(Clone, Copy, Debug)]
pub enum Instr {
    Nop,
    PushCopy(u16),
    PushConst(u16),
    PushCapture(u16),
    Swap(u16),
    Pop(u16),
    Jump(i16),
    JumpIf(i16),
    UnOp(UnOp),
    BinOp(BinOp),
    NewList(u16),
    NewFunc(u16),
}

#[derive(Clone)]
pub struct Func {
    pub arity: usize,
    pub instrs: Vec<Instr>,
    pub consts: Vec<Value>,
    pub captures: Vec<Value>,
}

impl Default for Func {
    fn default() -> Func {
        Func::new(0)
    }
}

impl Func {
    pub fn new(arity: usize) -> Func {
        Func {
            arity,
            instrs: Vec::new(),
            consts: Vec::new(),
            captures: Vec::new(),
        }
    }

    pub fn add_const(&mut self, val: Value) -> u16 {
        let id = u16::try_from(self.consts.len()).expect("too many constants");
        self.consts.push(val);
        id
    }

    pub fn add_instr(&mut self, instr: Instr) -> usize {
        let idx = self.instrs.len();
        self.instrs.push(instr);
        idx
    }
}

impl Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "fn({} args, {} captures):",
            self.arity,
            self.captures.len()
        )?;

        let mut f = indented(f);

        for (i, val) in self.consts.iter().enumerate() {
            writeln!(f, "{}: {:?}", i, val)?;
        }

        for (i, instr) in self.instrs.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }

            write!(f, "{:?}", instr)?;
        }

        Ok(())
    }
}
