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
    PushFunc(u16),
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

#[derive(Default)]
pub struct Vm {
    stack: Vec<Value>,
    callstack: Vec<Arc<Func>>,
    ip: usize,
}

impl Vm {
    pub fn new() -> Vm {
        Vm::default()
    }

    pub fn eval(&mut self, func: Arc<Func>) -> Value {
        self.callstack.push(func);
        self.run();
        self.stack.pop().unwrap()
    }

    fn run(&mut self) {
        let func = self.callstack[self.callstack.len() - 1].clone();
        self.ip = 0;
        loop {
            let instr = func.instrs[self.ip];
            self.ip += 1;

            if matches!(instr, Instr::Ret) {
                break;
            }

            self.dispatch(&func, instr);
        }

        self.callstack.pop();
    }

    fn dispatch(&mut self, func: &Func, instr: Instr) {
        match instr {
            Instr::Nop => self.instr_nop(),
            Instr::PushCopy(v) => self.instr_push_copy(v),
            Instr::PushConst(v) => self.instr_push_const(func, v),
            Instr::PushCapture(v) => self.instr_push_capture(func, v),
            Instr::PushFunc(v) => self.instr_push_func(v),
            Instr::PopSwap(v) => self.instr_pop_swap(v),
            Instr::Call => self.instr_call(),
            Instr::Ret => self.instr_ret(),
            Instr::Jump(v) => self.instr_jump(v),
            Instr::JumpIf(v) => self.instr_jump_if(v),
            Instr::UnOp(v) => self.instr_un_op(v),
            Instr::BinOp(v) => self.instr_bin_op(v),
            Instr::NewList(v) => self.instr_new_list(v),
            Instr::NewFunc(v) => self.instr_new_func(v),
        }
    }

    fn instr_nop(&mut self) {}

    fn instr_push_copy(&mut self, offset: u16) {
        let idx = self.stack.len() - usize::from(offset) - 1;
        self.stack.push(self.stack[idx].clone());
    }

    fn instr_push_const(&mut self, func: &Func, idx: u16) {
        self.stack.push(func.consts[usize::from(idx)].clone());
    }

    fn instr_push_capture(&mut self, func: &Func, idx: u16) {
        self.stack.push(func.captures[usize::from(idx)].clone());
    }

    fn instr_push_func(&mut self, offset: u16) {
        let func = self.callstack[self.callstack.len() - usize::from(offset) - 1].clone();
        self.stack.push(Value::Func(func));
    }

    fn instr_pop_swap(&mut self, count: u16) {
        let idx = self.stack.len() - usize::from(count) - 1;
        let last = self.stack.len() - 1;
        self.stack.swap(idx, last);
        for _ in 0..count {
            self.stack.pop();
        }
    }

    fn instr_call(&mut self) {
        match self.stack.pop() {
            Some(Value::Func(func)) => {
                let old_ip = self.ip;
                self.callstack.push(func);
                self.run();
                self.ip = old_ip;
            }
            _ => panic!("not a func"),
        }
    }

    fn instr_ret(&mut self) {}

    fn instr_jump(&mut self, offset: i16) {
        self.ip = (self.ip as isize + isize::from(offset)) as usize;
    }

    fn instr_jump_if(&mut self, offset: i16) {
        if let Some(value) = self.stack.pop() {
            if value.is_true() {
                self.ip = (self.ip as isize + isize::from(offset)) as usize;
            }
        }
    }

    fn instr_un_op(&mut self, _: UnOp) {
        todo!()
    }

    fn instr_bin_op(&mut self, op: BinOp) {
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.pop().unwrap();
        let res = lhs.bin_op(&rhs, op);
        self.stack.push(res);
    }

    fn instr_new_func(&mut self, num_captures: u16) {
        let mut func = match self.stack.pop() {
            Some(Value::Func(func)) => func,
            _ => panic!("not a func"),
        };

        let func_ref = Arc::make_mut(&mut func);

        func_ref.captures.clear();

        for _ in 0..num_captures {
            func_ref.captures.push(self.stack.pop().unwrap());
        }

        func_ref.captures.reverse();

        self.stack.push(Value::Func(func));
    }

    fn instr_new_list(&mut self, _: u16) {
        todo!()
    }
}
