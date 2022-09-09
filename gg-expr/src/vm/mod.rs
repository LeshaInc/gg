mod consts;
mod instr;
mod ops;
mod reg;

pub use self::consts::{CompiledConsts, ConstId, Consts};
pub use self::instr::{CompiledInstrs, Instr, InstrIdx, InstrOffset, Instrs};
use self::reg::Regs;
pub use self::reg::{RegId, RegSeq, RegSeqIter};
use crate::syntax::{BinOp, UnOp};
use crate::{Func, FuncValue, Value};

#[derive(Debug, Default)]
pub struct Vm {
    stack: Vec<Value>,
    callstack: Vec<Frame>,
}

#[derive(Debug)]
struct Frame {
    ip: InstrIdx,
    func_idx: usize,
    dst: usize,
}

impl Vm {
    pub fn new() -> Vm {
        Vm::default()
    }

    pub fn eval(&mut self, func: FuncValue) -> Value {
        let num_slots = func.slots;

        self.stack.push(Value::null());
        self.stack.push(func.into());

        for _ in 0..num_slots {
            self.stack.push(Value::null());
        }

        self.callstack.push(Frame {
            ip: InstrIdx(0),
            func_idx: 1,
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
                let func = &self.stack[frame.func_idx].as_func().unwrap();
                let instr = func.instrs[frame.ip];
                frame.ip += InstrOffset(1);

                let base = self.stack.len() - usize::from(func.slots);
                let (lhs, rhs) = self.stack.split_at_mut(base);
                let func = &lhs[frame.func_idx].as_func().unwrap();
                let regs = Regs(rhs);

                match instr {
                    Instr::Call { seq, dst } => {
                        self.instr_call(seq, dst);
                        break;
                    }
                    Instr::Ret { arg } => {
                        let ret = regs[arg].clone();
                        for _ in 0..func.slots {
                            self.stack.pop();
                        }

                        self.stack[frame.dst] = ret;
                        self.callstack.pop();
                        break;
                    }
                    _ => {}
                }

                let mut cur = CurrentFrame {
                    func,
                    ip: frame.ip,
                    regs,
                };

                match instr {
                    Instr::Panic => cur.instr_panic(),
                    Instr::Copy { src, dst } => cur.instr_copy(src, dst),
                    Instr::LoadConst { src, dst } => cur.instr_load_const(src, dst),
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

    fn instr_call(&mut self, seq: RegSeq, dst: RegId) {
        let frame = self.callstack.last().unwrap();
        let func = self.stack[frame.func_idx].as_func().unwrap();

        let base = self.stack.len() - usize::from(func.slots);
        let dst = base + usize::from(dst.0);

        let func_idx = base + usize::from(seq.base.0);
        let func = self.stack[func_idx].as_func().unwrap();

        let mut rem_slots = func.slots;

        for arg in seq.into_iter().skip(1) {
            let val = self.stack[base + usize::from(arg.0)].clone();
            self.stack.push(val);
            rem_slots -= 1;
        }

        for _ in 0..rem_slots {
            self.stack.push(Value::null());
        }

        self.callstack.push(Frame {
            ip: InstrIdx(0),
            func_idx,
            dst,
        });
    }
}

struct CurrentFrame<'a> {
    func: &'a Func,
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

    fn instr_load_const(&mut self, src: ConstId, dst: RegId) {
        self.regs[dst] = self.func.consts[src].clone();
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
        let lhs = &self.regs[lhs];
        let rhs = &self.regs[rhs];
        self.regs[dst] = ops::bin_op(op, lhs, rhs).unwrap();
    }

    fn instr_un_op(&mut self, op: UnOp, arg: RegId, dst: RegId) {
        let arg = &self.regs[arg];
        self.regs[dst] = ops::un_op(op, arg).unwrap();
    }
}
