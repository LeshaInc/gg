use crate::{Spanned, Token};

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i32),
    Float(f32),
    BinOp(BinOpExpr),
    UnOp(UnOpExpr),
    Error,
}

#[derive(Clone, Debug)]
pub struct BinOpExpr {
    pub lhs: Box<Spanned<Expr>>,
    pub op: BinOp,
    pub rhs: Box<Spanned<Expr>>,
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
        use BinOp::*;

        Some(match token {
            Token::Or => Or,
            Token::And => And,
            Token::Lt => Lt,
            Token::Le => Le,
            Token::Eq => Eq,
            Token::Neq => Neq,
            Token::Ge => Ge,
            Token::Gt => Gt,
            Token::Add => Add,
            Token::Sub => Sub,
            Token::Mul => Mul,
            Token::Div => Div,
            Token::Rem => Rem,
            Token::Pow => Pow,
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

#[derive(Clone, Debug)]
pub struct UnOpExpr {
    pub op: UnOp,
    pub expr: Box<Spanned<Expr>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

impl UnOp {
    pub fn from_token(token: Token) -> Option<UnOp> {
        use UnOp::*;

        Some(match token {
            Token::Sub => Neg,
            Token::Not => Not,
            _ => return None,
        })
    }

    pub fn binding_power(self) -> u8 {
        12
    }
}
