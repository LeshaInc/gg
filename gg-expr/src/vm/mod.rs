mod consts;
mod instr;
mod ops;
mod reg;

pub use self::consts::{CompiledConsts, ConstId, Consts};
pub use self::instr::{CompiledInstrs, Instr, InstrIdx, InstrOffset, Instrs};
use self::reg::Regs;
pub use self::reg::{RegId, RegSeq, RegSeqIter};
use crate::syntax::{BinOp, UnOp};
use crate::{FuncValue, Value};

#[derive(Debug, Default)]
pub struct Vm {
    stack: Vec<Value>,
    callstack: Vec<Frame>,
}

#[derive(Debug)]
struct Frame {
    ip: InstrIdx,
    func: FuncValue,
    dst: usize,
}

impl Vm {
    pub fn new() -> Vm {
        Vm::default()
    }

    pub fn eval(&mut self, func: FuncValue) -> Value {
        self.stack.push(Value::null());

        for _ in 0..func.slots {
            self.stack.push(Value::null());
        }

        for (i, val) in func.consts.0.iter().enumerate() {
            self.stack[i + 1] = val.clone();
        }

        self.callstack.push(Frame {
            ip: InstrIdx(0),
            func,
            dst: 0,
        });

        self.run();

        let value = self.stack.swap_remove(0);
        self.stack.clear();

        value
    }

    fn run(&mut self) {
        while let Some(frame) = self.callstack.last_mut() {
            loop {
                let instr = frame.func.instrs[frame.ip];
                frame.ip += InstrOffset(1);

                let start_idx = self.stack.len() - usize::from(frame.func.slots);
                let regs = Regs(&mut self.stack[start_idx..]);

                match instr {
                    Instr::Call { seq, dst } => {
                        self.instr_call(seq, dst);
                        break;
                    }
                    Instr::Ret { arg } => {
                        self.stack[frame.dst] = regs[arg].clone();

                        for _ in 0..frame.func.slots {
                            self.stack.pop();
                        }

                        self.callstack.pop();
                        break;
                    }
                    _ => {}
                }

                let mut cur = CurrentFrame { ip: frame.ip, regs };

                match instr {
                    Instr::Panic => cur.instr_panic(),
                    Instr::Copy { src, dst } => cur.instr_copy(src, dst),
                    Instr::NewList { seq, dst } => cur.instr_new_list(seq, dst),
                    Instr::NewMap { seq, dst } => cur.instr_new_map(seq, dst),
                    Instr::Jump { offset } => cur.instr_jump(offset),
                    Instr::JumpIfTrue { cond, offset } => cur.instr_jump_if_true(cond, offset),
                    Instr::JumpIfFalse { cond, offset } => cur.instr_jump_if_false(cond, offset),
                    Instr::BinOp { op, lhs, rhs, dst } => cur.instr_bin_op(op, lhs, rhs, dst),
                    Instr::UnOp { op, arg, dst } => cur.instr_un_op(op, arg, dst),
                    Instr::Nop | Instr::Call { .. } | Instr::Ret { .. } => {}
                }

                frame.ip = cur.ip;
            }
        }
    }

    fn instr_call(&mut self, _seq: RegSeq, _dst: RegId) {
        todo!()
    }
}

struct CurrentFrame<'a> {
    ip: InstrIdx,
    regs: Regs<'a>,
}

impl CurrentFrame<'_> {
    fn instr_panic(&mut self) {
        panic!("vm panicked");
    }

    fn instr_copy(&mut self, src: RegId, dst: RegId) {
        self.regs[dst] = self.regs[src].clone();
    }

    fn instr_new_list(&mut self, _seq: RegSeq, _dst: RegId) {
        todo!()
    }

    fn instr_new_map(&mut self, _seq: RegSeq, _dst: RegId) {
        todo!()
    }

    fn instr_jump(&mut self, offset: InstrOffset) {
        self.ip += offset;
    }

    fn instr_jump_if_true(&mut self, cond: RegId, offset: InstrOffset) {
        if self.regs[cond].is_truthy() {
            self.ip += offset;
        }
    }

    fn instr_jump_if_false(&mut self, cond: RegId, offset: InstrOffset) {
        if !self.regs[cond].is_truthy() {
            self.ip += offset;
        }
    }

    fn instr_bin_op(&mut self, op: BinOp, lhs: RegId, rhs: RegId, dst: RegId) {
        self.regs[dst] = ops::bin_op(op, &self.regs[lhs], &self.regs[rhs]).unwrap();
    }

    fn instr_un_op(&mut self, op: UnOp, arg: RegId, dst: RegId) {
        self.regs[dst] = ops::un_op(op, &self.regs[arg]).unwrap();
    }
}
