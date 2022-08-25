use std::fmt::{self, Display};

use super::SyntaxKind::{self, *};

macro_rules! define_op {
    (pub enum $ty:ident { $($name:ident($token:pat, $repr:expr),)* }) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
        pub enum $ty {
             $($name,)*
        }

        impl $ty {
            pub const VALUES: [$ty ; define_op!(@len $($name,)*)] = [
                $($ty ::$name,)*
            ];

            pub fn from_token(token: SyntaxKind) -> Option<$ty > {
                match token {
                    $($token => Some($ty ::$name),)*
                    _ => None,
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
        Or(TokOr, "||"),
        Coalesce(TokCoalesce, "??"),
        And(TokAnd, "&&"),
        Lt(TokLt, "<"),
        Le(TokLe, "<="),
        Eq(TokEq, "=="),
        Neq(TokNeq, "!="),
        Ge(TokGe, ">="),
        Gt(TokGt, ">"),
        Add(TokAdd, "+"),
        Sub(TokSub, "-"),
        Mul(TokMul, "*"),
        Div(TokDiv, "/"),
        Rem(TokRem, "%"),
        Pow(TokPow, "**"),
        Index(TokDot | TokLBracket, "[]"),
        IndexNullable(TokQuestionDot | TokQuestionLBracket, "?[]"),
    }
}

define_op! {
    pub enum UnOp {
        Neg(TokSub, "-"),
        Not(TokNot, "!"),
    }
}
