pub mod consts;
pub mod instr;
pub mod reg;

use self::consts::Consts;
use self::instr::{Instr, Instrs};
use self::reg::{RegAlloc, RegId};
use crate::syntax::*;
use crate::Value;

pub struct Compiler {
    regs: RegAlloc,
    instrs: Instrs,
    consts: Consts,
}

impl Compiler {
    pub fn compile_expr(&mut self, expr: Expr) -> RegId {
        match expr {
            Expr::Null(expr) => self.compile_expr_null(expr),
            Expr::Bool(expr) => self.compile_expr_bool(expr),
            Expr::Int(expr) => self.compile_expr_int(expr),
            Expr::Float(expr) => self.compile_expr_float(expr),
            Expr::String(expr) => self.compile_expr_string(expr),
            Expr::Binding(expr) => self.compile_expr_binding(expr),
            Expr::Binary(expr) => self.compile_expr_binary(expr),
            Expr::Unary(expr) => self.compile_expr_unary(expr),
            Expr::Grouped(expr) => self.compile_expr_grouped(expr),
            Expr::List(expr) => self.compile_expr_list(expr),
            Expr::Map(expr) => self.compile_expr_map(expr),
            Expr::Call(expr) => self.compile_expr_call(expr),
            Expr::Index(expr) => self.compile_expr_index(expr),
            Expr::IfElse(expr) => self.compile_expr_if_else(expr),
            Expr::LetIn(expr) => self.compile_expr_let_in(expr),
            Expr::Match(expr) => self.compile_expr_match(expr),
            Expr::Fn(expr) => self.compile_expr_fn(expr),
        }
    }

    fn compile_const(&mut self, value: impl Into<Value>) -> RegId {
        let res = self.regs.alloc();
        let id = self.consts.add(value.into());
        self.instrs.add(Instr::LoadConst { id, res });
        res
    }

    fn compile_expr_null(&mut self, _expr: ExprNull) -> RegId {
        self.compile_const(Value::null())
    }

    fn compile_expr_bool(&mut self, expr: ExprBool) -> RegId {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value)
    }

    fn compile_expr_int(&mut self, expr: ExprInt) -> RegId {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value)
    }

    fn compile_expr_float(&mut self, expr: ExprFloat) -> RegId {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value)
    }

    fn compile_expr_string(&mut self, expr: ExprString) -> RegId {
        let value = expr.value().unwrap_or_default();
        self.compile_const(value)
    }

    fn compile_expr_binding(&mut self, _expr: ExprBinding) -> RegId {
        todo!()
    }

    fn compile_expr_binary(&mut self, _expr: ExprBinary) -> RegId {
        todo!()
    }

    fn compile_expr_unary(&mut self, _expr: ExprUnary) -> RegId {
        todo!()
    }

    fn compile_expr_grouped(&mut self, _expr: ExprGrouped) -> RegId {
        todo!()
    }

    fn compile_expr_list(&mut self, _expr: ExprList) -> RegId {
        todo!()
    }

    fn compile_expr_map(&mut self, _expr: ExprMap) -> RegId {
        todo!()
    }

    fn compile_expr_call(&mut self, _expr: ExprCall) -> RegId {
        todo!()
    }

    fn compile_expr_index(&mut self, _expr: ExprIndex) -> RegId {
        todo!()
    }

    fn compile_expr_if_else(&mut self, _expr: ExprIfElse) -> RegId {
        todo!()
    }

    fn compile_expr_let_in(&mut self, _expr: ExprLetIn) -> RegId {
        todo!()
    }

    fn compile_expr_match(&mut self, _expr: ExprMatch) -> RegId {
        todo!()
    }

    fn compile_expr_fn(&mut self, _expr: ExprFn) -> RegId {
        todo!()
    }
}
