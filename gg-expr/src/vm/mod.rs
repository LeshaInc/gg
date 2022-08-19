mod bin_op;

use crate::syntax::{BinOp, UnOp};
use crate::{Func, Value};

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
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

    pub fn eval(&mut self, func: Value) -> Value {
        self.ipstack.push(0);
        self.callstack.push(func);
        self.run();
        self.stack.pop().unwrap()
    }

    fn run(&mut self) {
        'outer: while self.callstack.len() > 0 {
            let func = self.callstack[self.callstack.len() - 1].clone();
            let func = func.as_func().unwrap();
            self.ip = self.ipstack[self.ipstack.len() - 1];

            'inner: loop {
                let instr = func.instructions[self.ip];
                self.ip += 1;

                self.dispatch(&func, instr);

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
    }

    fn dispatch(&mut self, func: &Func, instr: Instruction) {
        match instr {
            Instruction::Nop => self.instr_nop(),
            Instruction::PushCopy(v) => self.instr_push_copy(v),
            Instruction::PushConst(v) => self.instr_push_const(func, v),
            Instruction::PushCapture(v) => self.instr_push_capture(func, v),
            Instruction::PushFunc(v) => self.instr_push_func(v),
            Instruction::PopSwap(v) => self.instr_pop_swap(v),
            Instruction::Call => self.instr_call(),
            Instruction::Ret => self.instr_ret(),
            Instruction::Jump(v) => self.instr_jump(v),
            Instruction::JumpIf(v) => self.instr_jump_if(v),
            Instruction::UnOp(v) => self.instr_un_op(v),
            Instruction::BinOp(v) => self.instr_bin_op(v),
            Instruction::NewList(v) => self.instr_new_list(v),
            Instruction::NewFunc(v) => self.instr_new_func(v),
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
        self.stack.push(func);
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
        let func = self.stack.pop().unwrap();
        self.callstack.push(func);
        self.ipstack.pop();
        self.ipstack.push(self.ip);
        self.ipstack.push(0);
    }

    fn instr_ret(&mut self) {}

    fn instr_jump(&mut self, offset: i16) {
        self.ip = (self.ip as isize + isize::from(offset)) as usize;
    }

    fn instr_jump_if(&mut self, offset: i16) {
        if let Some(value) = self.stack.pop() {
            if value.is_truthy() {
                self.ip = (self.ip as isize + isize::from(offset)) as usize;
            }
        }
    }

    fn instr_un_op(&mut self, _: UnOp) {
        todo!()
    }

    fn instr_bin_op(&mut self, op: BinOp) {
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.last_mut().unwrap();
        *lhs = bin_op::bin_op(&lhs, &rhs, op);
    }

    fn instr_new_func(&mut self, num_captures: u16) {
        let mut value = self.stack.pop().unwrap();

        let func = value.as_func_mut().unwrap();
        func.captures.clear();

        for _ in 0..num_captures {
            func.captures.push(self.stack.pop().unwrap());
        }

        func.captures.reverse();

        self.stack.push(value);
    }

    fn instr_new_list(&mut self, _: u16) {
        todo!()
    }
}
