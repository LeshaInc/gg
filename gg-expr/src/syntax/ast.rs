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
    Paren(Box<Spanned<Expr>>),
    List(ListExpr),
    Map(MapExpr),
    Func(FuncExpr),
    Call(CallExpr),
    IfElse(IfElseExpr),
    LetIn(LetInExpr),
    Error,
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
            Expr::Paren(v) => write!(f, "({})", v),
            Expr::List(v) => v.fmt(f),
            Expr::Map(v) => v.fmt(f),
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
        match self.op {
            BinOp::Index => write!(f, "{}[{}]", self.lhs, self.rhs),
            BinOp::IndexNullable => write!(f, "{}?[{}]", self.lhs, self.rhs),
            _ => write!(f, "{} {} {}", self.lhs, self.op, self.rhs),
        }
    }
}

macro_rules! define_op {
    (pub enum $ty:ident { $($name:ident($token:ident, $repr:expr),)* }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        pub enum $ty {
             $($name,)*
        }

        impl $ty {
            pub const VALUES: [$ty ; define_op!(@len $($name,)*)] = [
                $($ty ::$name,)*
            ];

            #[allow(unreachable_patterns)]
            pub fn from_token(token: Token) -> Option<$ty > {
                match token {
                    $(Token::$token => Some($ty ::$name),)*
                    _ => None,
                }
            }

            pub fn into_token(self) -> Token {
                match self {
                    $($ty::$name => Token::$token,)*
                }
            }
        }

        impl Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(match self {
                    $($ty::$name => $repr,)*
                })
            }
        }
    };

    (@len) => {
        0
    };

    (@len $first:ident, $($rest:ident,)*) => {
        1 + define_op!(@len $($rest,)*)
    }
}

define_op! {
    pub enum BinOp {
        Or(Or, "||"),
        Coalesce(Coalesce, "??"),
        And(And, "&&"),
        Lt(Lt, "<"),
        Le(Le, "<="),
        Eq(Eq, "=="),
        Neq(Neq, "!="),
        Ge(Ge, ">="),
        Gt(Gt, ">"),
        Add(Add, "+"),
        Sub(Sub, "-"),
        Mul(Mul, "*"),
        Div(Div, "/"),
        Rem(Rem, "%"),
        Pow(Pow, "**"),
        Index(LBracket, "[]"),
        IndexNullable(QuestionLBracket, "?[]"),
    }
}

#[derive(Clone, Debug)]
pub struct UnOpExpr {
    pub op: UnOp,
    pub expr: Box<Spanned<Expr>>,
}

impl Display for UnOpExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.op, self.expr)
    }
}

define_op! {
    pub enum UnOp {
        Neg(Sub, "-"),
        Not(Not, "!"),
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
pub struct MapExpr {
    pub pairs: Vec<(MapKey, Spanned<Expr>)>,
}

#[derive(Clone, Debug)]
pub enum MapKey {
    Ident(Spanned<String>),
    Expr(Spanned<Expr>),
}

impl Display for MapExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;

        for (i, (k, v)) in self.pairs.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }

            match k {
                MapKey::Ident(k) => write!(f, "{} = {}", k, v)?,
                MapKey::Expr(k) => write!(f, "[{}] = {}", k, v)?,
            }
        }

        write!(f, "}}")
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
        write!(f, "{}(", self.func)?;

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
