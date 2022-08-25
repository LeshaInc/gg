mod ops;

use std::fmt::Write;

use crate::diagnostic::{Diagnostic, Severity, SourceComponent};
use crate::syntax::{BinOp, TextRange, UnOp};
use crate::{DebugInfo, Error, Func, Value};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Instruction {
    Nop,
    Panic,
    PushCopy(u32),
    PushConst(u32),
    PushCapture(u32),
    PushFunc(u32),
    PopSwap(u32),
    Call,
    Ret,
    Jump(i32),
    JumpIf(i32),
    UnOp(UnOp),
    BinOp(BinOp),
    NewList(u32),
    NewMap(u32),
    NewFunc(u32),
}

#[derive(Default)]
pub struct Vm {
    stack: Vec<Value>,
    callstack: Vec<Value>,
    ipstack: Vec<usize>,
    ip: usize,
}

impl Vm {
    pub fn new() -> Vm {
        Vm::default()
    }

    pub fn eval(&mut self, func: &Value, args: &[Value]) -> Result<Value, Error> {
        self.ipstack.push(0);
        self.callstack.push(func.clone());
        self.stack.extend(args.iter().cloned());
        self.run()?;
        Ok(self.stack.pop().unwrap()) // TODO
    }

    fn get_current_debug_info(&self) -> Option<&DebugInfo> {
        let value = self.callstack.last().unwrap();
        let func = value.as_func().unwrap();
        func.debug_info.as_deref()
    }

    fn get_current_ranges(&self) -> &[TextRange] {
        match self.get_current_debug_info() {
            Some(info) => &info.instruction_ranges[self.ip - 1],
            None => &[],
        }
    }

    fn run(&mut self) -> Result<(), Error> {
        'outer: while !self.callstack.is_empty() {
            let value = self.callstack[self.callstack.len() - 1].clone();
            let value = if let Ok(thunk) = value.as_thunk() {
                thunk.force_eval()?
            } else {
                &value
            };

            let func = value.as_func().unwrap();

            self.ip = self.ipstack[self.ipstack.len() - 1];

            'inner: loop {
                let instr = func.instructions[self.ip];
                self.ip += 1;

                self.dispatch(func, instr)?;

                if matches!(instr, Instruction::Ret) {
                    break 'inner;
                }

                if matches!(instr, Instruction::Call) {
                    continue 'outer;
                }
            }

            self.callstack.pop();
            self.ipstack.pop();
        }

        Ok(())
    }

    fn dispatch(&mut self, func: &Func, instr: Instruction) -> Result<(), Error> {
        match instr {
            Instruction::Nop => self.instr_nop(),
            Instruction::Panic => self.instr_panic(),
            Instruction::PushCopy(v) => self.instr_push_copy(v),
            Instruction::PushConst(v) => self.instr_push_const(func, v),
            Instruction::PushCapture(v) => self.instr_push_capture(func, v),
            Instruction::PushFunc(v) => self.instr_push_func(v),
            Instruction::PopSwap(v) => self.instr_pop_swap(v),
            Instruction::Call => self.instr_call(),
            Instruction::Ret => self.instr_ret(),
            Instruction::Jump(v) => self.instr_jump(v),
            Instruction::JumpIf(v) => self.instr_jump_if(v),
            Instruction::UnOp(v) => self.instr_un_op(v)?,
            Instruction::BinOp(v) => self.instr_bin_op(v)?,
            Instruction::NewList(v) => self.instr_new_list(v),
            Instruction::NewMap(v) => self.instr_new_map(v),
            Instruction::NewFunc(v) => self.instr_new_func(v),
        }

        Ok(())
    }

    fn instr_nop(&mut self) {}

    fn instr_panic(&mut self) {
        panic!("vm panicked!");
    }

    fn instr_push_copy(&mut self, offset: u32) {
        let idx = self.stack.len() - (offset as usize) - 1;
        self.stack.push(self.stack[idx].clone());
    }

    fn instr_push_const(&mut self, func: &Func, idx: u32) {
        self.stack.push(func.consts[(idx as usize)].clone());
    }

    fn instr_push_capture(&mut self, func: &Func, idx: u32) {
        self.stack.push(func.captures[(idx as usize)].clone());
    }

    fn instr_push_func(&mut self, offset: u32) {
        let func = self.callstack[self.callstack.len() - (offset as usize) - 1].clone();
        self.stack.push(func);
    }

    fn instr_pop_swap(&mut self, count: u32) {
        let idx = self.stack.len() - (count as usize) - 1;
        let last = self.stack.len() - 1;
        self.stack.swap(idx, last);
        for _ in 0..count {
            self.stack.pop();
        }
    }

    fn instr_call(&mut self) {
        let func = self.stack.pop().unwrap();
        self.callstack.push(func);
        self.ipstack.pop();
        self.ipstack.push(self.ip);
        self.ipstack.push(0);
    }

    fn instr_ret(&mut self) {}

    fn instr_jump(&mut self, offset: i32) {
        self.ip = (self.ip as isize + (offset as isize)) as usize;
    }

    fn instr_jump_if(&mut self, offset: i32) {
        if let Some(value) = self.stack.pop() {
            if value.is_truthy() {
                self.ip = (self.ip as isize + (offset as isize)) as usize;
            }
        }
    }

    fn instr_un_op(&mut self, op: UnOp) -> Result<(), Error> {
        let val = self.stack.last_mut().unwrap();
        match ops::un_op(val, op) {
            Some(v) => {
                *val = v;
                Ok(())
            }
            None => {
                let val = self.stack.pop().unwrap();
                Err(self.error_un_op(val, op))
            }
        }
    }

    #[cold]
    fn error_un_op(&self, val: Value, op: UnOp) -> Error {
        let message = format!(
            "unary operator `{}` cannot be applied to `{:?}`",
            op,
            val.ty(),
        );

        let diagnostic = Diagnostic::new(Severity::Error, message);

        let debug_info = match self.get_current_debug_info() {
            Some(v) => v,
            None => return Error::new(diagnostic),
        };

        let ranges = self.get_current_ranges();

        let source = SourceComponent::new(debug_info.source.clone()).with_label(
            Severity::Error,
            ranges[1],
            format!("`{:?}`", val.ty()),
        );

        Error::new(diagnostic.with_source(source))
    }

    fn instr_bin_op(&mut self, op: BinOp) -> Result<(), Error> {
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.last_mut().unwrap();
        match ops::bin_op(lhs, &rhs, op) {
            Some(v) => {
                *lhs = v;
                Ok(())
            }
            None => {
                let lhs = self.stack.pop().unwrap();
                Err(self.error_bin_op(lhs, rhs, op))
            }
        }
    }

    #[cold]
    fn error_bin_op(&self, lhs: Value, rhs: Value, op: BinOp) -> Error {
        if op == BinOp::Index {
            if lhs.is_list() {
                return self.error_list_index_oob(lhs, rhs);
            } else if lhs.is_map() {
                return self.error_no_such_key(lhs, rhs);
            }
        }

        let message = format!(
            "operator `{}` cannot be applied to `{:?}` and `{:?}`",
            op,
            lhs.ty(),
            rhs.ty()
        );

        let diagnostic = Diagnostic::new(Severity::Error, message);

        let debug_info = match self.get_current_debug_info() {
            Some(v) => v,
            None => return Error::new(diagnostic),
        };

        let ranges = self.get_current_ranges();

        let source = SourceComponent::new(debug_info.source.clone())
            .with_label(Severity::Error, ranges[1], format!("`{:?}`", lhs.ty()))
            .with_label(Severity::Error, ranges[2], format!("`{:?}`", rhs.ty()));

        Error::new(diagnostic.with_source(source))
    }

    #[cold]
    fn error_list_index_oob(&self, lhs: Value, rhs: Value) -> Error {
        let lhs = lhs.as_list().unwrap();
        let rhs = rhs.as_int().unwrap();

        let message = if !lhs.is_empty() {
            format!(
                "list index out of bounds: {} not in [0, {})",
                rhs,
                lhs.len()
            )
        } else {
            "list index out of bounds: attempt to index an empty list".to_string()
        };

        let diagnostic = Diagnostic::new(Severity::Error, message);

        let debug_info = match self.get_current_debug_info() {
            Some(v) => v,
            None => return Error::new(diagnostic),
        };

        let ranges = self.get_current_ranges();

        let source = SourceComponent::new(debug_info.source.clone())
            .with_label(Severity::Error, ranges[1], format!("length: {}", lhs.len()))
            .with_label(Severity::Error, ranges[2], format!("index: {}", rhs));

        Error::new(diagnostic.with_source(source))
    }

    #[cold]
    fn error_no_such_key(&self, lhs: Value, rhs: Value) -> Error {
        let lhs = lhs.as_map().unwrap();

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

        let mut diagnostic = Diagnostic::new(Severity::Error, message);

        let debug_info = match self.get_current_debug_info() {
            Some(v) => v,
            None => return Error::new(diagnostic),
        };

        let ranges = self.get_current_ranges();

        diagnostic = diagnostic.with_source(
            SourceComponent::new(debug_info.source.clone())
                .with_label(Severity::Error, ranges[1], lhs_label)
                .with_label(Severity::Error, ranges[2], rhs_label),
        );

        if let Ok(str) = rhs.as_string() {
            let mut keys = lhs.keys().flat_map(|k| k.as_string()).collect::<Vec<_>>();
            keys.sort_by_cached_key(|v| strsim::damerau_levenshtein(v, str));

            let mut help = String::from("perhaps you meant ");

            for (i, var) in keys.iter().take(3).enumerate() {
                if i > 0 {
                    help.push_str(", ");
                }

                let _ = write!(&mut help, "{:?}", var);
            }

            if !keys.is_empty() {
                diagnostic = diagnostic.with_help(help);
            }
        }

        Error::new(diagnostic)
    }

    fn instr_new_func(&mut self, num_captures: u32) {
        let mut value = self.stack.pop().unwrap();

        let func = value.as_func_mut().unwrap();
        func.captures.clear();

        for _ in 0..num_captures {
            func.captures.push(self.stack.pop().unwrap());
        }

        func.captures.reverse();

        self.stack.push(value);
    }

    fn instr_new_list(&mut self, count: u32) {
        let mut list = im::Vector::new();

        for _ in 0..count {
            list.push_front(self.stack.pop().unwrap());
        }

        self.stack.push(list.into());
    }

    fn instr_new_map(&mut self, count: u32) {
        let mut map = im::HashMap::new();

        for _ in 0..count {
            let value = self.stack.pop().unwrap();
            let key = self.stack.pop().unwrap();
            map.insert(key, value);
        }

        self.stack.push(map.into());
    }
}
