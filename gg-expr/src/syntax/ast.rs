use std::fmt::{self, Display};
use std::sync::Arc;

use super::{Spanned, Token};

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i64),
    Float(f64),
    String(Arc<String>),
    Var(String),
    BinOp(BinOpExpr),
    UnOp(UnOpExpr),
    List(ListExpr),
    Func(FuncExpr),
    Call(CallExpr),
    IfElse(IfElseExpr),
    LetIn(LetInExpr),
    Error,
}

impl Expr {
    pub fn binding_power(&self) -> (u8, u8) {
        match self {
            Expr::UnOp(v) => (255, v.op.binding_power()),
            Expr::BinOp(v) => v.op.binding_power(),
            Expr::Func(_) | Expr::IfElse(_) | Expr::LetIn(_) => (0, 0),
            _ => (255, 255),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Int(v) => v.fmt(f),
            Expr::Float(v) => v.fmt(f),
            Expr::String(v) => write!(f, "{:?}", v),
            Expr::Var(v) => v.fmt(f),
            Expr::BinOp(v) => v.fmt(f),
            Expr::UnOp(v) => v.fmt(f),
            Expr::List(v) => v.fmt(f),
            Expr::Func(v) => v.fmt(f),
            Expr::Call(v) => v.fmt(f),
            Expr::IfElse(v) => v.fmt(f),
            Expr::LetIn(v) => v.fmt(f),
            Expr::Error => write!(f, "error"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BinOpExpr {
    pub lhs: Box<Spanned<Expr>>,
    pub op: BinOp,
    pub rhs: Box<Spanned<Expr>>,
}

impl Display for BinOpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (l_bp, r_bp) = self.op.binding_power();

        if l_bp > self.lhs.item.binding_power().1 {
            write!(f, "({})", self.lhs)?;
        } else {
            write!(f, "{}", self.lhs)?;
        }

        write!(f, " {} ", self.op)?;

        if r_bp > self.rhs.item.binding_power().0 {
            write!(f, "({})", self.rhs)?;
        } else {
            write!(f, "{}", self.rhs)?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinOp {
    Or,

    And,

    Lt,
    Le,
    Eq,
    Neq,
    Ge,
    Gt,

    Add,
    Sub,

    Mul,
    Div,
    Rem,

    Pow,
}

impl BinOp {
    pub fn from_token(token: Token) -> Option<BinOp> {
        Some(match token {
            Token::Or => BinOp::Or,
            Token::And => BinOp::And,
            Token::Lt => BinOp::Lt,
            Token::Le => BinOp::Le,
            Token::Eq => BinOp::Eq,
            Token::Neq => BinOp::Neq,
            Token::Ge => BinOp::Ge,
            Token::Gt => BinOp::Gt,
            Token::Add => BinOp::Add,
            Token::Sub => BinOp::Sub,
            Token::Mul => BinOp::Mul,
            Token::Div => BinOp::Div,
            Token::Rem => BinOp::Rem,
            Token::Pow => BinOp::Pow,
            _ => return None,
        })
    }

    pub fn binding_power(self) -> (u8, u8) {
        use BinOp::*;

        match self {
            Or => (1, 2),
            And => (3, 4),
            Lt | Le | Eq | Neq | Ge | Gt => (5, 6),
            Add | Sub => (7, 8),
            Mul | Div | Rem => (9, 10),
            Pow => (14, 13),
        }
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BinOp::*;

        f.write_str(match self {
            Or => "||",
            And => "&&",
            Lt => "<",
            Le => "<=",
            Eq => "==",
            Neq => "!=",
            Ge => ">=",
            Gt => ">",
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
            Rem => "%",
            Pow => "**",
        })
    }
}

#[derive(Clone, Debug)]
pub struct UnOpExpr {
    pub op: UnOp,
    pub expr: Box<Spanned<Expr>>,
}

impl Display for UnOpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bp = self.op.binding_power();

        if bp > self.expr.item.binding_power().0 {
            write!(f, "{}({})", self.op, self.expr)
        } else {
            write!(f, "{}{}", self.op, self.expr)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

impl UnOp {
    pub fn from_token(token: Token) -> Option<UnOp> {
        Some(match token {
            Token::Sub => UnOp::Neg,
            Token::Not => UnOp::Not,
            _ => return None,
        })
    }

    pub fn binding_power(self) -> u8 {
        12
    }
}

impl Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
        })
    }
}

#[derive(Clone, Debug)]
pub struct ListExpr {
    pub exprs: Vec<Spanned<Expr>>,
}

impl Display for ListExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;

        for (i, v) in self.exprs.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }

            v.fmt(f)?;
        }

        write!(f, "]")
    }
}

#[derive(Clone, Debug)]
pub struct FuncExpr {
    pub args: Vec<String>,
    pub expr: Box<Spanned<Expr>>,
}

impl Display for FuncExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn(")?;

        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }

            write!(f, "{}", arg)?;
        }

        write!(f, "): {}", self.expr)
    }
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    pub func: Box<Spanned<Expr>>,
    pub args: Vec<Spanned<Expr>>,
}

impl Display for CallExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (bp, _) = self.func.item.binding_power();
        if bp < 255 {
            write!(f, "({})(", self.func)?;
        } else {
            write!(f, "{}(", self.func)?;
        }

        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }

            write!(f, "{}", arg)?;
        }

        write!(f, ")")
    }
}

#[derive(Clone, Debug)]
pub struct IfElseExpr {
    pub cond: Box<Spanned<Expr>>,
    pub if_true: Box<Spanned<Expr>>,
    pub if_false: Box<Spanned<Expr>>,
}

impl Display for IfElseExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "if {} then {} else {}",
            self.cond, self.if_true, self.if_false
        )
    }
}

#[derive(Clone, Debug)]
pub struct LetInExpr {
    pub vars: Vec<(String, Box<Spanned<Expr>>)>,
    pub expr: Box<Spanned<Expr>>,
}

impl Display for LetInExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "let ")?;

        for (i, (var, expr)) in self.vars.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }

            write!(f, "{} = {}", var, expr)?;
        }

        write!(f, " in {}", self.expr)
    }
}
