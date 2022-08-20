mod bin_op;

use crate::syntax::{BinOp, UnOp};
use crate::{Func, Value};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

    pub fn eval(&mut self, func: &Value, args: &[Value]) -> Value {
        self.ipstack.push(0);
        self.callstack.push(func.clone());
        self.stack.extend(args.iter().cloned());
        self.run();
        self.stack.pop().unwrap()
    }

    fn run(&mut self) {
        'outer: while self.callstack.len() > 0 {
            let value = self.callstack[self.callstack.len() - 1].clone();
            let value = if let Ok(thunk) = value.as_thunk() {
                thunk.force_eval()
            } else {
                &value
            };

            let func = value.as_func().unwrap();

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
            Instruction::UnOp(v) => self.instr_un_op(v),
            Instruction::BinOp(v) => self.instr_bin_op(v),
            Instruction::NewList(v) => self.instr_new_list(v),
            Instruction::NewFunc(v) => self.instr_new_func(v),
        }
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

    fn instr_un_op(&mut self, _: UnOp) {
        todo!()
    }

    fn instr_bin_op(&mut self, op: BinOp) {
        let rhs = self.stack.pop().unwrap();
        let lhs = self.stack.last_mut().unwrap();
        *lhs = bin_op::bin_op(&lhs, &rhs, op);
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

    fn instr_new_list(&mut self, _: u32) {
        todo!()
    }
}
