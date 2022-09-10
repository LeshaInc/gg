mod consts;
mod error;
mod instr;
mod ops;
mod reg;

use std::sync::Arc;

pub use self::consts::{CompiledConsts, ConstId, Consts};
pub use self::error::{Error, Result, StackFrame, StackTrace};
pub use self::instr::{CompiledInstrs, Instr, InstrIdx, InstrOffset, Instrs, Opcode};
pub use self::reg::{RegId, RegSeq, RegSeqIter};
use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::{BinOp, TextRange, UnOp};
use crate::{Func, FuncValue, Source, Value};

#[derive(Debug, Default)]
pub struct Vm {
    frames: Vec<Frame>,
    stack: Vec<Value>,
}

#[derive(Debug)]
struct VmContext {
    frame: Frame,
    frames: Vec<Frame>,
    stack: Vec<Value>,
}

#[derive(Debug)]
struct Frame {
    ip: InstrIdx,
    func: usize,
    dst: usize,
    base: usize,
}

impl Vm {
    pub fn new() -> Vm {
        Vm::default()
    }

    pub fn eval(&mut self, func: &Value, args: &[&Value]) -> Result<Value> {
        let mut rem_slots = func.as_func().unwrap().slots;

        self.stack.push(Value::null());
        self.stack.push(func.clone());

        for &arg in args {
            self.stack.push(arg.clone());
            rem_slots -= 1;
        }

        for _ in 0..rem_slots {
            self.stack.push(Value::null());
        }

        self.frames.push(Frame {
            ip: InstrIdx(0),
            func: 1,
            dst: 0,
            base: 2,
        });

        self.run()?;

        let value = self.stack.remove(0);
        self.stack.clear();

        Ok(value)
    }

    fn run(&mut self) -> Result<()> {
        let frame = self.frames.pop().unwrap();
        let mut ctx = VmContext {
            frame,
            frames: std::mem::take(&mut self.frames),
            stack: std::mem::take(&mut self.stack),
        };

        while ctx.frame.ip != InstrIdx(u32::MAX) {
            let instr = ctx.fetch()?;
            ctx.dispatch(instr)?;
        }

        self.frames = ctx.frames;
        self.stack = ctx.stack;

        Ok(())
    }
}

impl VmContext {
    fn stack_trace(&self, range: Option<TextRange>) -> StackTrace {
        let mut frames = Vec::with_capacity(self.frames.len() + 1);

        frames.push(StackFrame {
            range,
            func: self.stack[self.frame.func].clone().try_into().unwrap(),
        });

        for frame in self.frames.iter().rev() {
            let func = FuncValue::try_from(self.stack[frame.func].clone()).unwrap();

            let range = func.debug_info.as_ref().and_then(|di| {
                let prev_ip = &(frame.ip + InstrOffset(-1));
                di.instruction_ranges
                    .get(&prev_ip)
                    .and_then(|v| v.get(0).copied())
            });

            frames.push(StackFrame { range, func });
        }

        StackTrace { frames }
    }

    fn cur_ranges(&self) -> Option<Vec<TextRange>> {
        if let Some(di) = &self.cur_func().debug_info {
            let prev_ip = &(self.frame.ip + InstrOffset(-1));
            return di.instruction_ranges.get(&prev_ip).cloned();
        }
        None
    }

    fn error(
        &self,
        range: Option<TextRange>,
        message: impl Into<String>,
        extra: impl FnOnce(&mut Diagnostic, Option<Arc<Source>>),
    ) -> Error {
        let debug_info = self.cur_func().debug_info.as_ref();
        let source = debug_info.map(|v| v.source.clone());

        let mut diagnostic = Diagnostic::new(Severity::Error, message.into());
        extra(&mut diagnostic, source);

        Error::new(diagnostic).with_stack_trace(self.stack_trace(range))
    }

    fn cur_func(&self) -> &Func {
        self.stack[self.frame.func].as_func().unwrap()
    }

    fn reg_offset(&self, id: RegId) -> Result<usize> {
        let idx = self.frame.base + usize::from(id.0);
        if idx >= self.stack.len() {
            Err(self.reg_invalid())
        } else {
            Ok(idx)
        }
    }

    #[cold]
    fn reg_invalid(&self) -> Error {
        Diagnostic::new(Severity::Error, "invalid register").into()
    }

    fn reg_read(&self, id: RegId) -> Result<&Value> {
        let idx = self.reg_offset(id)?;
        Ok(&self.stack[idx])
    }

    fn reg_write(&mut self, id: RegId, value: Value) -> Result<Value> {
        let idx = self.reg_offset(id)?;
        Ok(std::mem::replace(&mut self.stack[idx], value))
    }

    fn const_read(&self, id: ConstId) -> Result<&Value> {
        let func = self.cur_func();
        func.consts
            .0
            .get(usize::from(id.0))
            .ok_or_else(|| self.const_invalid())
    }

    #[cold]
    fn const_invalid(&self) -> Error {
        Diagnostic::new(Severity::Error, "invalid register").into()
    }

    fn fetch(&mut self) -> Result<Instr> {
        let func = self.cur_func();
        let instrs = &func.instrs.0;
        let instr = instrs
            .get(self.frame.ip.0 as usize)
            .copied()
            .ok_or_else(|| self.code_overrun())?;
        self.frame.ip += InstrOffset(1);
        Ok(instr)
    }

    #[cold]
    fn code_overrun(&self) -> Error {
        Diagnostic::new(Severity::Error, "code overrun").into()
    }

    fn dispatch(&mut self, instr: Instr) -> Result<()> {
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
            Opcode::Call => self.instr_call(instr),
            Opcode::Ret => self.instr_ret(instr),
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

    fn instr_nop(&mut self, _instr: Instr) -> Result<()> {
        Ok(())
    }

    fn instr_panic(&mut self, _instr: Instr) -> Result<()> {
        Err(Diagnostic::new(Severity::Error, "vm panicked").into())
    }

    fn instr_copy(&mut self, instr: Instr) -> Result<()> {
        let val = self.reg_read(instr.reg_a())?;
        self.reg_write(instr.reg_b(), val.clone())?;
        Ok(())
    }

    fn instr_load_const(&mut self, instr: Instr) -> Result<()> {
        let val = self.const_read(instr.const_id())?;
        self.reg_write(instr.reg_b(), val.clone())?;
        Ok(())
    }

    fn instr_new_list(&mut self, _instr: Instr) -> Result<()> {
        todo!()
    }

    fn instr_new_map(&mut self, _instr: Instr) -> Result<()> {
        todo!()
    }

    fn instr_jump(&mut self, instr: Instr) -> Result<()> {
        self.frame.ip += instr.offset();
        Ok(())
    }

    fn instr_jump_if_true(&mut self, instr: Instr) -> Result<()> {
        let val = self.reg_read(instr.reg_a())?;
        if val.is_truthy() {
            self.frame.ip += instr.offset();
        }
        Ok(())
    }

    fn instr_jump_if_false(&mut self, instr: Instr) -> Result<()> {
        let val = self.reg_read(instr.reg_a())?;
        if !val.is_truthy() {
            self.frame.ip += instr.offset();
        }
        Ok(())
    }

    fn instr_call(&mut self, instr: Instr) -> Result<()> {
        let (func_reg, args) = instr.reg_seq().split_first();
        let dst_reg = instr.reg_c();

        let func_val = self.reg_read(func_reg)?;
        let func = func_val
            .as_func()
            .map_err(|_| self.error(None, "not a function", |_, _| ()))?;

        let old_base = self.frame.base;
        let new_base = self.stack.len();

        self.push_nulls(usize::from(func.slots));

        for (i, arg) in args.into_iter().enumerate() {
            let src = old_base + usize::from(arg.0);
            let dst = new_base + i;
            self.stack.swap(src, dst);
        }

        let new_frame = Frame {
            ip: InstrIdx(0),
            func: old_base + usize::from(func_reg.0),
            dst: old_base + usize::from(dst_reg.0),
            base: new_base,
        };

        let old_frame = std::mem::replace(&mut self.frame, new_frame);
        self.frames.push(old_frame);

        Ok(())
    }

    fn push_nulls(&mut self, count: usize) {
        unsafe {
            self.stack.reserve(count);
            std::ptr::write_bytes(self.stack.as_mut_ptr().add(self.stack.len()), 0, count);
            self.stack.set_len(self.stack.len() + count);
        }
    }

    fn instr_ret(&mut self, instr: Instr) -> Result<()> {
        let val = self.reg_write(instr.reg_a(), Value::null())?;

        let cur_func = self.cur_func();
        let num_slots = cur_func.slots;
        let dst = self.frame.dst;

        for _ in 0..num_slots {
            self.stack.pop();
        }

        self.stack[dst] = val;

        if let Some(v) = self.frames.pop() {
            self.frame = v;
        } else {
            self.frame.ip = InstrIdx(u32::MAX);
        }

        Ok(())
    }

    fn instr_op_or(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Or)
    }

    fn instr_op_coalesce(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Coalesce)
    }

    fn instr_op_and(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::And)
    }

    fn instr_op_lt(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Lt)
    }

    fn instr_op_le(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Le)
    }

    fn instr_op_eq(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Eq)
    }

    fn instr_op_neq(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Neq)
    }

    fn instr_op_ge(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Ge)
    }

    fn instr_op_gt(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Gt)
    }

    fn instr_op_add(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Add)
    }

    fn instr_op_sub(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Sub)
    }

    fn instr_op_mul(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Mul)
    }

    fn instr_op_div(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Div)
    }

    fn instr_op_rem(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Rem)
    }

    fn instr_op_pow(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Pow)
    }

    fn instr_op_index(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::Index)
    }

    fn instr_op_index_nullable(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, BinOp::IndexNullable)
    }

    fn instr_un_op_neg(&mut self, instr: Instr) -> Result<()> {
        self.instr_un_op(instr, UnOp::Neg)
    }

    fn instr_un_op_not(&mut self, instr: Instr) -> Result<()> {
        self.instr_un_op(instr, UnOp::Not)
    }

    #[inline(always)]
    fn instr_bin_op(&mut self, instr: Instr, op: BinOp) -> Result<()> {
        let lhs = self.reg_read(instr.reg_a())?;
        let rhs = self.reg_read(instr.reg_b())?;
        let res = ops::bin_op(op, lhs, rhs).ok_or_else(|| self.error_bin_op(lhs, rhs, op))?;
        self.reg_write(instr.reg_c(), res)?;
        Ok(())
    }

    #[inline(always)]
    fn instr_un_op(&mut self, instr: Instr, op: UnOp) -> Result<()> {
        let arg = self.reg_read(instr.reg_a())?;
        let res = ops::un_op(op, arg).ok_or_else(|| self.error_un_op(arg, op))?;
        self.reg_write(instr.reg_b(), res)?;
        Ok(())
    }

    #[cold]
    fn error_bin_op(&self, lhs: &Value, rhs: &Value, op: BinOp) -> Error {
        let message = format!(
            "operator `{}` cannot be applied to `{:?}` and `{:?}`",
            op,
            lhs.ty(),
            rhs.ty()
        );

        let ranges = self.cur_ranges();
        let main_range = ranges.as_ref().map(|v| v[0]);

        self.error(main_range, message, |diag, source| {
            if let (Some(source), Some(ranges)) = (source, ranges) {
                diag.add_source(
                    SourceComponent::new(source)
                        .with_label(Severity::Error, ranges[1], format!("`{:?}`", lhs.ty()))
                        .with_label(Severity::Error, ranges[2], format!("`{:?}`", rhs.ty())),
                );
            }
        })
    }

    #[cold]
    fn error_un_op(&self, arg: &Value, op: UnOp) -> Error {
        let message = format!(
            "unary operator `{}` cannot be applied to `{:?}`",
            op,
            arg.ty(),
        );

        let ranges = self.cur_ranges();
        let main_range = ranges.as_ref().map(|v| v[0]);

        self.error(main_range, message, |diag, source| {
            if let (Some(source), Some(ranges)) = (source, ranges) {
                diag.add_source(SourceComponent::new(source).with_label(
                    Severity::Error,
                    ranges[1],
                    format!("`{:?}`", arg.ty()),
                ));
            }
        })
    }
}
