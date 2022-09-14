mod consts;
mod error;
mod instr;
mod reg;
mod upvalues;

use std::fmt::Write;
use std::sync::Arc;

pub use self::consts::{CompiledConsts, ConstId, Consts};
pub use self::error::{Error, Result, StackFrame, StackTrace};
pub use self::instr::{CompiledInstrs, Instr, InstrIdx, InstrOffset, Instrs, Opcode};
pub use self::reg::{RegId, RegSeq, RegSeqIter};
pub use self::upvalues::{UpfnId, UpvalueId, UpvalueNames, Upvalues};
use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::TextRange;
use crate::{Func, FuncValue, List, Map, Source, Value};

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
    base: usize,
    func: usize,
    dst: usize,
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
            base: 2,
            func: 1,
            dst: 0,
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
        if let Some(di) = &self.cur_func().ok()?.debug_info {
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
        let debug_info = self
            .stack
            .get(self.frame.func)
            .and_then(|v| v.as_func().ok())
            .and_then(|f| f.debug_info.as_ref());
        let source = debug_info.map(|v| v.source.clone());

        let mut diagnostic = Diagnostic::new(Severity::Error, message.into());
        extra(&mut diagnostic, source);

        Error::new(diagnostic).with_stack_trace(self.stack_trace(range))
    }

    #[inline(never)]
    fn error_simple(&self, message: &str) -> Error {
        self.error(None, message, |_, _| ())
    }

    fn cur_func(&self) -> Result<&Func> {
        self.stack
            .get(self.frame.func)
            .and_then(|v| v.as_func().ok())
            .ok_or_else(|| self.error_bad_fn())
    }

    #[inline(never)]
    fn error_bad_fn(&self) -> Error {
        self.error_simple("invalid function")
    }

    fn reg_offset(&self, id: RegId) -> Result<usize> {
        let idx = self.frame.base + usize::from(id.0);
        if idx >= self.stack.len() {
            Err(self.error_bad_reg())
        } else {
            Ok(idx)
        }
    }

    #[inline(never)]
    fn error_bad_reg(&self) -> Error {
        self.error_simple("invalid register")
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
        let func = self.cur_func()?;
        func.consts.get(id).ok_or_else(|| self.error_bad_const())
    }

    #[inline(never)]
    fn error_bad_const(&self) -> Error {
        self.error_simple("invalid constant")
    }

    fn upvalue_read(&self, id: UpvalueId) -> Result<&Value> {
        let func = self.cur_func()?;
        func.upvalues
            .get(id)
            .ok_or_else(|| self.error_bad_upvalue())
    }

    fn upfn_read(&self, id: UpfnId) -> Result<&Value> {
        let idx = if id.0 == 0 {
            self.frame.func
        } else {
            self.frames
                .get(id.0 as usize)
                .map(|frame| frame.func)
                .ok_or_else(|| self.error_bad_upvalue())?
        };

        self.stack.get(idx).ok_or_else(|| self.error_bad_upvalue())
    }

    #[inline(never)]
    fn error_bad_upvalue(&self) -> Error {
        self.error_simple("invalid upvalue")
    }

    fn fetch(&mut self) -> Result<Instr> {
        let func = self.cur_func()?;
        let instrs = &func.instrs.0;
        let instr = instrs
            .get(self.frame.ip.0 as usize)
            .copied()
            .ok_or_else(|| self.error_code_overrun())?;
        self.frame.ip += InstrOffset(1);
        Ok(instr)
    }

    #[inline(never)]
    fn error_code_overrun(&self) -> Error {
        self.error_simple("code overrun")
    }

    #[inline(always)]
    fn dispatch(&mut self, instr: Instr) -> Result<()> {
        match instr.opcode {
            Opcode::Nop => self.instr_nop(instr),
            Opcode::Panic => self.instr_panic(instr),
            Opcode::LoadConst => self.instr_load_const(instr),
            Opcode::LoadUpvalue => self.instr_load_upvalue(instr),
            Opcode::LoadUpfn => self.instr_load_upfn(instr),
            Opcode::Copy => self.instr_copy(instr),
            Opcode::CopyIfTrue => self.instr_copy_if_true(instr),
            Opcode::NewList => self.instr_new_list(instr),
            Opcode::NewMap => self.instr_new_map(instr),
            Opcode::NewFunc => self.instr_new_func(instr),
            Opcode::Jump => self.instr_jump(instr),
            Opcode::JumpIfTrue => self.instr_jump_if_true(instr),
            Opcode::JumpIfFalse => self.instr_jump_if_false(instr),
            Opcode::Call => self.instr_call(instr),
            Opcode::TailCall => self.instr_tail_call(instr),
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
        Err(self.error_panic())
    }

    #[inline(never)]
    fn error_panic(&self) -> Error {
        self.error_simple("panic")
    }

    fn instr_copy(&mut self, instr: Instr) -> Result<()> {
        let val = self.reg_read(instr.reg_a())?;
        self.reg_write(instr.reg_b(), val.clone())?;
        Ok(())
    }

    fn instr_copy_if_true(&mut self, instr: Instr) -> Result<()> {
        let cond = self.reg_read(instr.reg_c())?;
        if cond.is_truthy() {
            let val = self.reg_read(instr.reg_a())?;
            self.reg_write(instr.reg_b(), val.clone())?;
        }
        Ok(())
    }

    fn instr_load_const(&mut self, instr: Instr) -> Result<()> {
        let val = self.const_read(instr.const_id())?;
        self.reg_write(instr.reg_b(), val.clone())?;
        Ok(())
    }

    fn instr_load_upvalue(&mut self, instr: Instr) -> Result<()> {
        let val = self.upvalue_read(instr.upvalue_id())?;
        self.reg_write(instr.reg_b(), val.clone())?;
        Ok(())
    }

    fn instr_load_upfn(&mut self, instr: Instr) -> Result<()> {
        let val = self.upfn_read(instr.upfn_id())?;
        self.reg_write(instr.reg_b(), val.clone())?;
        Ok(())
    }

    fn instr_new_list(&mut self, instr: Instr) -> Result<()> {
        let mut list = List::new();

        for reg in instr.reg_seq() {
            let value = self.reg_read(reg)?;
            list.push_back(value.clone());
        }

        self.reg_write(instr.reg_c(), list.into())?;

        Ok(())
    }

    fn instr_new_map(&mut self, instr: Instr) -> Result<()> {
        let mut map = Map::new();

        for reg in instr.reg_seq().into_iter().step_by(2) {
            let key = self.reg_read(reg)?;
            let value = self.reg_read(RegId(reg.0 + 1))?;
            map.insert(key.clone(), value.clone());
        }

        self.reg_write(instr.reg_c(), map.into())?;

        Ok(())
    }

    fn instr_new_func(&mut self, instr: Instr) -> Result<()> {
        let (fn_reg, ups_regs) = instr.reg_seq().split_first();
        let func = self
            .reg_read(fn_reg)?
            .as_func()
            .map_err(|_| self.error_bad_fn())?;

        let mut ups = vec![Value::null(); ups_regs.len as usize];

        for (up, up_reg) in ups.iter_mut().zip(ups_regs) {
            *up = self.reg_read(up_reg)?.clone();
        }

        let func = Func {
            arity: func.arity,
            slots: func.slots,
            instrs: func.instrs.clone(),
            consts: func.consts.clone(),
            upvalues: Upvalues(ups.into()),
            debug_info: func.debug_info.clone(),
        };

        self.reg_write(instr.reg_c(), Value::from(func))?;

        Ok(())
    }

    fn instr_jump(&mut self, instr: Instr) -> Result<()> {
        self.frame.ip += instr.offset();
        Ok(())
    }

    fn instr_jump_if_true(&mut self, instr: Instr) -> Result<()> {
        let cond = self.reg_read(instr.reg_a())?;
        if cond.is_truthy() {
            self.frame.ip += instr.offset();
        }
        Ok(())
    }

    fn instr_jump_if_false(&mut self, instr: Instr) -> Result<()> {
        let cond = self.reg_read(instr.reg_a())?;
        if !cond.is_truthy() {
            self.frame.ip += instr.offset();
        }
        Ok(())
    }

    const MAX_DEPTH: usize = 1024;

    fn instr_call(&mut self, instr: Instr) -> Result<()> {
        if self.frames.len() == Self::MAX_DEPTH {
            return Err(self.error_stack_overflow());
        }

        let seq = instr.reg_seq();
        let (func_reg, arg_regs) = seq.split_first();

        let dst_reg = instr.reg_c();

        let func_val = self.reg_read(func_reg)?;
        let func = func_val.as_func().map_err(|_| self.error_bad_fn())?;

        let old_base = self.frame.base;
        let new_base = self.stack.len();

        self.push_nulls(usize::from(func.slots));

        for (i, arg) in arg_regs.into_iter().enumerate() {
            let src = old_base + usize::from(arg.0);
            let dst = new_base + i;
            self.stack.swap(src, dst);
        }

        let new_frame = Frame {
            ip: InstrIdx(0),
            base: new_base,
            dst: old_base + usize::from(dst_reg.0),
            func: old_base + usize::from(func_reg.0),
        };

        let old_frame = std::mem::replace(&mut self.frame, new_frame);
        self.frames.push(old_frame);

        Ok(())
    }

    fn instr_tail_call(&mut self, instr: Instr) -> Result<()> {
        let seq = instr.reg_seq();
        let (func_reg, arg_regs) = seq.split_first();

        let func_val = self.reg_write(func_reg, Value::null())?;
        let func = func_val.as_func().map_err(|_| self.error_bad_fn())?;

        let base = self.frame.base;

        let cur_slots = self.stack.len() - base;
        let req_slots = usize::from(func.slots) + 1;
        if cur_slots < req_slots {
            self.push_nulls(req_slots - cur_slots);
        }

        for (i, arg) in arg_regs.into_iter().enumerate() {
            let src = base + usize::from(arg.0);
            let dst = base + i;
            self.stack.swap(src, dst);
        }

        self.frame.ip = InstrIdx(0);
        self.frame.func = self.stack.len() - 1;
        self.stack[self.frame.func] = func_val;

        Ok(())
    }

    #[cold]
    fn error_stack_overflow(&self) -> Error {
        self.error_simple("stack overflow")
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
        let dst = self.frame.dst;

        while self.stack.len() > self.frame.base {
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

    fn instr_bin_op(
        &mut self,
        instr: Instr,
        op: impl FnOnce(&VmContext, &Value, &Value) -> Result<Value>,
    ) -> Result<()> {
        let lhs = self.reg_read(instr.reg_a())?;
        let rhs = self.reg_read(instr.reg_b())?;
        let res = op(self, lhs, rhs)?;
        self.reg_write(instr.reg_c(), res)?;
        Ok(())
    }

    fn instr_un_op(
        &mut self,
        instr: Instr,
        op: impl FnOnce(&VmContext, &Value) -> Result<Value>,
    ) -> Result<()> {
        let arg = self.reg_read(instr.reg_a())?;
        let res = op(self, arg)?;
        self.reg_write(instr.reg_b(), res)?;
        Ok(())
    }

    #[inline(never)]
    fn error_bin_op(&self, instr: Instr) -> Error {
        let lhs = self.reg_read(instr.reg_a()).unwrap();
        let rhs = self.reg_read(instr.reg_b()).unwrap();

        let message = format!(
            "operator `{}` cannot be applied to `{:?}` and `{:?}`",
            instr.opcode.operator(),
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

    #[inline(never)]
    fn error_un_op(&self, instr: Instr) -> Error {
        let arg = self.reg_read(instr.reg_a()).unwrap();

        let message = format!(
            "unary operator `{}` cannot be applied to `{:?}`",
            instr.opcode.operator(),
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

    fn instr_op_or(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |_, x, y| Ok((x.is_truthy() || y.is_truthy()).into()))
    }

    fn instr_op_coalesce(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |_, x, y| {
            Ok((if x.is_null() { y } else { x }).clone())
        })
    }

    fn instr_op_and(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |_, x, y| Ok((x.is_truthy() && y.is_truthy()).into()))
    }

    fn instr_op_index(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |s, x, y| {
            let val = if let (Ok(x), Ok(y)) = (x.as_list(), y.as_int()) {
                usize::try_from(y)
                    .ok()
                    .and_then(|idx| x.get(idx))
                    .ok_or_else(|| s.error_list_oob(instr))?
            } else if let Ok(map) = x.as_map() {
                map.get(y).ok_or_else(|| s.error_no_such_key(instr))?
            } else {
                return Err(s.error_bin_op(instr));
            };

            Ok(val.clone())
        })
    }

    #[cold]
    fn error_list_oob(&self, instr: Instr) -> Error {
        let lhs = self.reg_read(instr.reg_a()).unwrap().as_list().unwrap();
        let rhs = self.reg_read(instr.reg_b()).unwrap().as_int().unwrap();

        let message = if !lhs.is_empty() {
            format!(
                "list index out of bounds: {} not in [0, {})",
                rhs,
                lhs.len()
            )
        } else {
            "list index out of bounds: empty list".to_string()
        };

        let ranges = self.cur_ranges();
        let main_range = ranges.as_ref().map(|v| v[0]);

        self.error(main_range, message, |diag, source| {
            if let (Some(source), Some(ranges)) = (source, ranges) {
                diag.add_source(
                    SourceComponent::new(source)
                        .with_label(Severity::Error, ranges[1], format!("length: {}", lhs.len()))
                        .with_label(Severity::Error, ranges[2], format!("index: {}", rhs)),
                );
            }
        })
    }

    #[cold]
    fn error_no_such_key(&self, instr: Instr) -> Error {
        let lhs = self.reg_read(instr.reg_a()).unwrap().as_map().unwrap();
        let rhs = self.reg_read(instr.reg_b()).unwrap();

        let lhs_label = format!("map of {} entries", lhs.len());
        let (message, rhs_label) =
            if rhs.is_string() || rhs.is_int() || rhs.is_float() || rhs.is_bool() {
                (
                    format!("key not present in map: {:?}", rhs),
                    format!("key: {:?}", rhs),
                )
            } else {
                ("key not present in map".to_string(), "key".to_string())
            };

        let mut help = None;

        if let Ok(str) = rhs.as_string() {
            let mut keys = lhs.keys().flat_map(|k| k.as_string()).collect::<Vec<_>>();
            keys.sort_by_cached_key(|v| strsim::damerau_levenshtein(v, str));

            let mut new_help = String::from("perhaps you meant ");

            for (i, var) in keys.iter().take(3).enumerate() {
                if i > 0 {
                    new_help.push_str(", ");
                }

                let _ = write!(&mut new_help, "{:?}", var);
            }

            if !keys.is_empty() {
                help = Some(new_help);
            }
        }

        let ranges = self.cur_ranges();
        let main_range = ranges.as_ref().map(|v| v[0]);

        self.error(main_range, message, |diag, source| {
            if let (Some(source), Some(ranges)) = (source, ranges) {
                diag.add_source(
                    SourceComponent::new(source)
                        .with_label(Severity::Error, ranges[1], lhs_label)
                        .with_label(Severity::Error, ranges[2], rhs_label),
                );
            }

            if let Some(help) = help {
                diag.add_help(help)
            }
        })
    }

    fn instr_op_index_nullable(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |s, x, y| {
            let val = if let Ok(x) = x.as_list() {
                let idx = y.as_int().ok().and_then(|v| usize::try_from(v).ok());
                idx.and_then(|idx| x.get(idx))
                    .cloned()
                    .unwrap_or_else(Value::null)
            } else if let Ok(map) = x.as_map() {
                map.get(y).cloned().unwrap_or_else(Value::null)
            } else {
                return Err(s.error_bin_op(instr));
            };

            Ok(val)
        })
    }
}

macro_rules! op_cmp {
    ($self:ident, $instr:ident, $op:tt) => {
        $self.instr_bin_op($instr, |s, x, y| {
            let res = if let (Ok(x), Ok(y)) = (x.as_int(), y.as_int()) {
                x $op  y
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_int()) {
                x $op (y as f32)
            } else if let (Ok(x), Ok(y)) = (x.as_int(), y.as_float()) {
                (x as f32) $op y
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_float()) {
                x $op y
            } else if let (Ok(x), Ok(y)) = (x.as_string(), y.as_string()) {
                x $op y
            } else {
                return Err(s.error_bin_op($instr))
            };

            Ok(res.into())
        })
    };
}

impl VmContext {
    fn instr_op_lt(&mut self, instr: Instr) -> Result<()> {
        op_cmp!(self, instr, <)
    }

    fn instr_op_le(&mut self, instr: Instr) -> Result<()> {
        op_cmp!(self, instr, <=)
    }

    fn instr_op_ge(&mut self, instr: Instr) -> Result<()> {
        op_cmp!(self, instr, >=)
    }

    fn instr_op_gt(&mut self, instr: Instr) -> Result<()> {
        op_cmp!(self, instr, >)
    }

    fn instr_op_eq(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |_, x, y| Ok(Value::from(x == y)))
    }

    fn instr_op_neq(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |_, x, y| Ok(Value::from(x != y)))
    }
}

macro_rules! op_arith {
    ($self:ident, $instr:ident, $int:ident, $op:tt) => {
        $self.instr_bin_op($instr, |s, x, y| {
            let res = if let (Ok(x), Ok(y)) = (x.as_int(), y.as_int()) {
                (x.$int(y)).map(Value::from)
                    .unwrap_or_else(|| ((x as f32) $op (y as f32)).into())
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_int()) {
                (x $op (y as f32)).into()
            } else if let (Ok(x), Ok(y)) = (x.as_int(), y.as_float()) {
                ((x as f32) $op y).into()
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_float()) {
                (x $op y).into()
            } else {
                return Err(s.error_bin_op($instr))
            };

            Ok(res)
        })
    };
}

impl VmContext {
    fn instr_op_add(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |s, x, y| {
            let res = if let (Ok(x), Ok(y)) = (x.as_int(), y.as_int()) {
                (x.checked_add(y))
                    .map(Value::from)
                    .unwrap_or_else(|| ((x as f32) + (y as f32)).into())
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_int()) {
                (x + (y as f32)).into()
            } else if let (Ok(x), Ok(y)) = (x.as_int(), y.as_float()) {
                ((x as f32) + y).into()
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_float()) {
                (x + y).into()
            } else if let (Ok(x), Ok(y)) = (x.as_string(), y.as_string()) {
                let mut res = String::with_capacity(x.len() + y.len());
                res.push_str(x);
                res.push_str(y);
                res.into()
            } else if let (Ok(x), Ok(y)) = (x.as_list(), y.as_list()) {
                (x + y).into()
            } else {
                return Err(s.error_bin_op(instr));
            };

            Ok(res)
        })
    }

    fn instr_op_sub(&mut self, instr: Instr) -> Result<()> {
        op_arith!(self, instr, checked_sub, -)
    }

    fn instr_op_mul(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |s, x, y| {
            let res = if let (Ok(x), Ok(y)) = (x.as_int(), y.as_int()) {
                (x.checked_mul(y))
                    .map(Value::from)
                    .unwrap_or_else(|| ((x as f32) * (y as f32)).into())
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_int()) {
                (x * (y as f32)).into()
            } else if let (Ok(x), Ok(y)) = (x.as_int(), y.as_float()) {
                ((x as f32) * y).into()
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_float()) {
                (x * y).into()
            } else if let (Ok(x), Ok(y)) = (x.as_string(), y.as_int()) {
                if let Ok(y) = usize::try_from(y) {
                    x.repeat(y).into()
                } else {
                    "".into()
                }
            } else if let (Ok(x), Ok(y)) = (x.as_list(), y.as_int()) {
                let mut res = List::new();
                for _ in 0..y {
                    res.append(x.clone());
                }
                res.into()
            } else {
                return Err(s.error_bin_op(instr));
            };

            Ok(res)
        })
    }

    fn instr_op_div(&mut self, instr: Instr) -> Result<()> {
        op_arith!(self, instr, checked_div, /)
    }

    fn instr_op_rem(&mut self, instr: Instr) -> Result<()> {
        op_arith!(self, instr, checked_rem, %)
    }

    fn instr_op_pow(&mut self, instr: Instr) -> Result<()> {
        self.instr_bin_op(instr, |s, x, y| {
            let res = if let (Ok(x), Ok(y)) = (x.as_int(), y.as_int()) {
                if y > 0 {
                    x.checked_pow(y as u32)
                        .map(Value::from)
                        .unwrap_or_else(|| (x as f32).powi(y).into())
                } else {
                    (x as f32).powi(y).into()
                }
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_int()) {
                x.powi(y).into()
            } else if let (Ok(x), Ok(y)) = (x.as_int(), y.as_float()) {
                (x as f32).powf(y).into()
            } else if let (Ok(x), Ok(y)) = (x.as_float(), y.as_float()) {
                x.powf(y).into()
            } else {
                return Err(s.error_bin_op(instr));
            };

            Ok(res)
        })
    }

    fn instr_un_op_neg(&mut self, instr: Instr) -> Result<()> {
        self.instr_un_op(instr, |s, x| {
            let res = if let Ok(x) = x.as_int() {
                x.checked_neg()
                    .map(Value::from)
                    .unwrap_or_else(|| (-(x as f32)).into())
            } else if let Ok(x) = x.as_float() {
                (-x).into()
            } else {
                return Err(s.error_un_op(instr));
            };

            Ok(res)
        })
    }

    fn instr_un_op_not(&mut self, instr: Instr) -> Result<()> {
        self.instr_un_op(instr, |_, x| Ok((!x.is_truthy()).into()))
    }
}
