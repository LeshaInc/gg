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
    PopSwap(u16),
    Ret,
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

pub fn interpret(func: &Func, stack: &mut Vec<Value>) {
    let mut ip = 0;
    loop {
        let instr = func.instrs[ip];
        ip += 1;

        match instr {
            Instr::Nop => todo!(),
            Instr::PushCopy(offset) => {
                let idx = stack.len() - usize::from(offset) - 1;
                stack.push(stack[idx].clone());
            }
            Instr::PushConst(idx) => {
                stack.push(func.consts[usize::from(idx)].clone());
            }
            Instr::PushCapture(idx) => {
                stack.push(func.captures[usize::from(idx)].clone());
            }
            Instr::PopSwap(count) => {
                let idx = stack.len() - usize::from(count) - 1;
                let last = stack.len() - 1;
                stack.swap(idx, last);
                for _ in 0..count {
                    stack.pop();
                }
            }
            Instr::Ret => break,
            Instr::Jump(offset) => {
                ip = (ip as isize + isize::from(offset)) as usize;
            }
            Instr::JumpIf(offset) => {
                if let Some(value) = stack.pop() {
                    if value.is_true() {
                        ip = (ip as isize + isize::from(offset)) as usize;
                    }
                }
            }
            Instr::BinOp(op) => {
                let rhs = stack.pop().unwrap();
                let lhs = stack.pop().unwrap();
                let res = lhs.bin_op(&rhs, op);
                stack.push(res);
            }
            _ => todo!(),
        }
    }
}
