use std::sync::Arc;

use crate::syntax::{Expr, FuncExpr, Spanned};
use crate::value::Thunk;
use crate::vm::{Func, Instr, InstrOffset, StackPos};
use crate::Value;

pub fn compile(expr: &Spanned<Expr>) -> Value {
    match &expr.item {
        Expr::Int(v) => Value::Int(*v),
        Expr::Float(v) => Value::Float(*v),
        Expr::Func(func) => Value::Func(Arc::new(compile_func(func))),
        _ => compile_thunk(expr),
    }
}

fn compile_func(func: &FuncExpr) -> Func {
    let mut res = Func::new(func.args.len());

    let mut ctx = Context {
        args: &func.args,
        func: &mut res,
        stack_len: 0,
    };

    compile_expr(&mut ctx, &func.expr);

    res
}

struct Context<'a> {
    args: &'a [String],
    func: &'a mut Func,
    stack_len: u16,
}

fn compile_expr(ctx: &mut Context, expr: &Spanned<Expr>) {
    match &expr.item {
        Expr::Int(v) => {
            let id = ctx.func.add_const(Value::Int(*v));
            ctx.func.add_instr(Instr::PushConst(id));
        }
        Expr::Float(v) => {
            let id = ctx.func.add_const(Value::Float(*v));
            ctx.func.add_instr(Instr::PushConst(id));
        }
        Expr::Var(name) => {
            let idx = ctx.args.iter().position(|s| s == name).unwrap();
            let pos = StackPos(ctx.stack_len + (idx as u16));
            ctx.func.add_instr(Instr::PushCopy(pos));
        }
        Expr::BinOp(expr) => {
            compile_expr(ctx, &expr.lhs);
            compile_expr(ctx, &expr.rhs);
            ctx.stack_len -= 2;
            ctx.func.add_instr(Instr::BinOp(expr.op));
        }
        Expr::UnOp(expr) => {
            compile_expr(ctx, &expr.expr);
            ctx.stack_len -= 2;
            ctx.func.add_instr(Instr::UnOp(expr.op));
        }
        Expr::IfElse(expr) => {
            compile_expr(ctx, &expr.cond);

            let start = ctx.func.add_instr(Instr::Nop);
            compile_expr(ctx, &expr.if_false);
            let mid = ctx.func.add_instr(Instr::Nop);
            compile_expr(ctx, &expr.if_true);
            let end = ctx.func.instrs.len();

            let offset = i16::try_from(mid - start).expect("jump too far");
            ctx.func.instrs[start] = Instr::JumpIf(InstrOffset(offset));

            let offset = i16::try_from(end - mid - 1).expect("jump too far");
            ctx.func.instrs[mid] = Instr::Jump(InstrOffset(offset));
        }
        _ => todo!(),
    }

    ctx.stack_len += 1;
}

fn compile_thunk(expr: &Spanned<Expr>) -> Value {
    let mut func = Func::new(0);
    let mut ctx = Context {
        args: &[],
        func: &mut func,
        stack_len: 0,
    };

    compile_expr(&mut ctx, expr);

    Value::Thunk(Arc::new(Thunk::new(func)))
}
