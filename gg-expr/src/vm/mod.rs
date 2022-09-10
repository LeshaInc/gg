mod consts;
mod instr;
mod ops;
mod reg;

pub use self::consts::{CompiledConsts, ConstId, Consts};
pub use self::instr::{CompiledInstrs, Instr, InstrIdx, InstrOffset, Instrs, Opcode};
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

    pub fn eval(&mut self, func: &Value, args: &[&Value]) -> Value {
        let func = FuncValue::try_from(func.clone()).unwrap();
        let mut rem_slots = func.slots;

        self.stack.push(Value::null());
        self.stack.push(func.into());

        for &arg in args {
            self.stack.push(arg.clone());
            rem_slots -= 1;
        }

        for _ in 0..rem_slots {
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

                if instr.opcode == Opcode::Call {
                    self.instr_call(instr.reg_seq(), instr.reg_c());
                    break;
                }

                if instr.opcode == Opcode::Ret {
                    let ret = regs[instr.reg_a()].clone();
                    for _ in 0..func.slots {
                        self.stack.pop();
                    }

                    self.stack[frame.dst] = ret;
                    self.callstack.pop();
                    break;
                }

                let mut cur = CurrentFrame {
                    func,
                    ip: frame.ip,
                    regs,
                };

                cur.dispatch(instr);

                frame.ip = cur.ip;
            }
        }
    }

    fn instr_call(&mut self, seq: RegSeq, dst: RegId) {
        let frame = self.callstack.last().unwrap();
        let func = self.stack[frame.func_idx].as_func().unwrap();

        let old_base = self.stack.len() - usize::from(func.slots);
        let dst = old_base + usize::from(dst.0);

        let func_idx = old_base + usize::from(seq.base.0);
        let func = self.stack[func_idx].as_func().unwrap();

        let new_base = self.stack.len();

        for _ in 0..func.slots {
            self.stack.push(Value::null());
        }

        for (i, arg) in seq.into_iter().skip(1).enumerate() {
            let src = old_base + usize::from(arg.0);
            let dst = new_base + i;
            self.stack.swap(src, dst);
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
    fn dispatch(&mut self, instr: Instr) {
        match instr.opcode {
            Opcode::Nop => self.instr_nop(instr),
            Opcode::Panic => self.instr_panic(instr),
            Opcode::LoadConst => self.instr_load_const(instr),
            Opcode::Copy => self.instr_copy(instr),
            Opcode::NewList => self.instr_new_list(instr),
            Opcode::NewMap => self.instr_new_map(instr),
            Opcode::Jump => self.instr_jump(instr),
            Opcode::JumpIfTrue => self.instr_jump_if_true(instr),
            Opcode::JumpIfFalse => self.instr_jump_if_false(instr),
            Opcode::Call => self.instr_nop(instr),
            Opcode::Ret => self.instr_nop(instr),
            Opcode::OpOr => self.instr_op_or(instr),
            Opcode::OpCoalesce => self.instr_op_coalesce(instr),
            Opcode::OpAnd => self.instr_op_and(instr),
            Opcode::OpLt => self.instr_op_lt(instr),
            Opcode::OpLe => self.instr_op_le(instr),
            Opcode::OpEq => self.instr_op_eq(instr),
            Opcode::OpNeq => self.instr_op_neq(instr),
            Opcode::OpGe => self.instr_op_ge(instr),
            Opcode::OpGt => self.instr_op_gt(instr),
            Opcode::OpAdd => self.instr_op_add(instr),
            Opcode::OpSub => self.instr_op_sub(instr),
            Opcode::OpMul => self.instr_op_mul(instr),
            Opcode::OpDiv => self.instr_op_div(instr),
            Opcode::OpRem => self.instr_op_rem(instr),
            Opcode::OpPow => self.instr_op_pow(instr),
            Opcode::OpIndex => self.instr_op_index(instr),
            Opcode::OpIndexNullable => self.instr_op_index_nullable(instr),
            Opcode::UnOpNeg => self.instr_un_op_neg(instr),
            Opcode::UnOpNot => self.instr_un_op_not(instr),
        }
    }

    fn instr_nop(&mut self, _instr: Instr) {}

    fn instr_panic(&mut self, _instr: Instr) {
        panic!("vm panicked");
    }

    fn instr_copy(&mut self, instr: Instr) {
        self.regs[instr.reg_b()] = self.regs[instr.reg_a()].clone();
    }

    fn instr_load_const(&mut self, instr: Instr) {
        self.regs[instr.reg_b()] = self.func.consts[instr.const_id()].clone();
    }

    fn instr_new_list(&mut self, _instr: Instr) {
        todo!()
    }

    fn instr_new_map(&mut self, _instr: Instr) {
        todo!()
    }

    fn instr_jump(&mut self, instr: Instr) {
        self.ip += instr.offset();
    }

    fn instr_jump_if_true(&mut self, instr: Instr) {
        if self.regs[instr.reg_a()].is_truthy() {
            self.ip += instr.offset();
        }
    }

    fn instr_jump_if_false(&mut self, instr: Instr) {
        if !self.regs[instr.reg_a()].is_truthy() {
            self.ip += instr.offset();
        }
    }

    fn instr_op_or(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Or)
    }

    fn instr_op_coalesce(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Coalesce)
    }

    fn instr_op_and(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::And)
    }

    fn instr_op_lt(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Lt)
    }

    fn instr_op_le(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Le)
    }

    fn instr_op_eq(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Eq)
    }

    fn instr_op_neq(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Neq)
    }

    fn instr_op_ge(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Ge)
    }

    fn instr_op_gt(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Gt)
    }

    fn instr_op_add(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Add)
    }

    fn instr_op_sub(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Sub)
    }

    fn instr_op_mul(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Mul)
    }

    fn instr_op_div(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Div)
    }

    fn instr_op_rem(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Rem)
    }

    fn instr_op_pow(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Pow)
    }

    fn instr_op_index(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::Index)
    }

    fn instr_op_index_nullable(&mut self, instr: Instr) {
        self.instr_bin_op(instr, BinOp::IndexNullable)
    }

    fn instr_un_op_neg(&mut self, instr: Instr) {
        self.instr_un_op(instr, UnOp::Neg)
    }

    fn instr_un_op_not(&mut self, instr: Instr) {
        self.instr_un_op(instr, UnOp::Not)
    }

    fn instr_bin_op(&mut self, instr: Instr, op: BinOp) {
        let lhs = &self.regs[instr.reg_a()];
        let rhs = &self.regs[instr.reg_b()];
        self.regs[instr.reg_c()] = ops::bin_op(op, lhs, rhs).unwrap();
    }

    fn instr_un_op(&mut self, instr: Instr, op: UnOp) {
        let arg = &self.regs[instr.reg_a()];
        self.regs[instr.reg_b()] = ops::un_op(op, arg).unwrap();
    }
}
