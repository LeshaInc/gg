use std::fmt::{self, Debug, Write};
use std::sync::Arc;

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
    Call,
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
    pub instrs: Arc<[Instr]>,
    pub consts: Arc<[Value]>,
    pub captures: Vec<Value>,
}

impl Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fn(...):")?;

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
            Instr::Call => match stack.pop() {
                Some(Value::Func(func)) => {
                    interpret(&func, stack);
                }
                _ => panic!("not a func"),
            },
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
            Instr::NewFunc(captures) => {
                let mut func = match stack.pop() {
                    Some(Value::Func(func)) => func,
                    _ => panic!("not a func"),
                };

                let func_ref = Arc::make_mut(&mut func);

                func_ref.captures.clear();

                for _ in 0..captures {
                    func_ref.captures.push(stack.pop().unwrap());
                }

                func_ref.captures.reverse();

                stack.push(Value::Func(func));
            }
            _ => todo!("{:?}", instr),
        }
    }
}
