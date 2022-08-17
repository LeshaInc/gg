use crate::syntax::{Expr, FuncExpr, Spanned};
use crate::vm::{Const, Func, Instr, StackPos};
use crate::Value;

pub fn compile(expr: &Spanned<Expr>) -> Value {
    match &expr.item {
        Expr::Int(v) => Value::Int(*v),
        Expr::Float(v) => Value::Float(*v),
        Expr::Func(func) => Value::Func(compile_func(func)),
        _ => todo!(),
    }
}

fn compile_func(func: &FuncExpr) -> Func {
    let mut res = Func {
        arity: func.args.len(),
        instrs: Vec::new(),
        consts: Vec::new(),
    };

    let mut ctx = Context {
        args: &func.args,
        func: &mut res,
        stack_len: 0,
    };

    compile_func_expr(&mut ctx, &func.expr);

    res
}

struct Context<'a> {
    args: &'a [String],
    func: &'a mut Func,
    stack_len: u16,
}

fn compile_func_expr(ctx: &mut Context, expr: &Spanned<Expr>) {
    match &expr.item {
        Expr::Int(v) => {
            let id = ctx.func.add_const(Const::Int(*v));
            ctx.func.add_instr(Instr::PushConst(id));
        }
        Expr::Float(v) => {
            let id = ctx.func.add_const(Const::Float(*v));
            ctx.func.add_instr(Instr::PushConst(id));
        }
        Expr::Var(name) => {
            let idx = ctx.args.iter().position(|s| s == name).unwrap();
            let pos = StackPos(ctx.stack_len + (idx as u16));
            ctx.func.add_instr(Instr::PushCopy(pos));
        }
        Expr::BinOp(expr) => {
            compile_func_expr(ctx, &expr.lhs);
            compile_func_expr(ctx, &expr.rhs);
            ctx.stack_len -= 2;
            ctx.func.add_instr(Instr::BinOp(expr.op));
        }
        Expr::UnOp(expr) => {
            compile_func_expr(ctx, &expr.expr);
            ctx.stack_len -= 2;
            ctx.func.add_instr(Instr::UnOp(expr.op));
        }
        _ => todo!(),
    }

    ctx.stack_len += 1;
}
