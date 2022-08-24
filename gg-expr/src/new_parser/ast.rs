use std::fmt;

use super::{parser, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

macro_rules! define_terms {
    ($( $name:ident, )+) => {
        $(
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name {
            syntax: SyntaxNode,
        }

        impl $name {
            pub fn cast(syntax: SyntaxNode) -> Option<Self>
            where
                Self: Sized,
            {
                if syntax.kind() == SyntaxKind::$name {
                    Some(Self { syntax })
                } else {
                    None
                }
            }

            pub fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }

            pub fn tokens(&self) -> impl Iterator<Item = SyntaxToken> {
                self.syntax.children_with_tokens().flat_map(|v| match v {
                    SyntaxElement::Token(token) => Some(token),
                    _ => None,
                })
            }

            pub fn nontrivial_tokens(&self) -> impl Iterator<Item = SyntaxToken> {
                self.tokens().filter(|v| !v.kind().is_trivia())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.syntax.fmt(f)
            }
        }
        )+
    }
}

macro_rules! define_enum {
    ($name:ident { $( $varname:ident($varnode:ident), )+ }) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub enum $name {
            $( $varname($varnode), )+
        }

        impl $name {
            pub fn cast(syntax: SyntaxNode) -> Option<Self>
            where
                Self: Sized,
            {
                match syntax.kind() {
                    $( SyntaxKind::$varnode => Some($name::$varname($varnode::cast(syntax).unwrap())), )+
                    _ => None
                }
            }

            pub fn syntax(&self) -> &SyntaxNode {
                match self {
                    $( $name::$varname(v) => v.syntax(), )+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.syntax().fmt(f)
            }
        }
    }
}

macro_rules! define_single_children {
    ($( $parent:ident: $func:ident -> $child:ident, )+) => {
        $(
        impl $parent {
            pub fn $func(&self) -> Option<$child> {
                self.syntax.children().find_map($child::cast)
            }
        }
        )+
    }
}

macro_rules! define_multi_children {
    ($( $parent:ident: $func:ident -> $child:ident, )+) => {
        $(
        impl $parent {
            pub fn $func(&self) -> impl Iterator<Item = $child> + '_ {
                self.syntax.children().filter_map($child::cast)
            }
        }
        )+
    }
}

define_terms![
    ExprNull,
    ExprBool,
    ExprInt,
    ExprFloat,
    ExprString,
    ExprBinding,
    ExprBinary,
    ExprUnary,
    ExprGrouped,
    ExprList,
    ExprMap,
    ExprFn,
    ExprCall,
    ExprIndex,
    ExprIfElse,
    ExprLetIn,
    PatGrouped,
    PatOr,
    PatList,
    PatInt,
    PatString,
    PatRest,
    PatHole,
    PatBinding,
    MapPair,
    LetBinding,
    FnArgs,
    CallArgs,
];

define_enum!(Expr {
    Null(ExprNull),
    Bool(ExprBool),
    Int(ExprInt),
    Float(ExprFloat),
    String(ExprString),
    Binding(ExprBinding),
    Unary(ExprUnary),
    Grouped(ExprGrouped),
    List(ExprList),
    Map(ExprMap),
    Fn(ExprFn),
    Call(ExprCall),
    Index(ExprIndex),
    IfElse(ExprIfElse),
    LetIn(ExprLetIn),
});

define_enum!(Pat {
    Grouped(PatGrouped),
    Or(PatOr),
    List(PatList),
    Int(PatInt),
    String(PatString),
    Rest(PatRest),
    Hole(PatHole),
    Binding(PatBinding),
});

define_single_children! {
    ExprUnary: expr -> Expr,
    ExprGrouped: expr -> Expr,
    ExprFn: args -> FnArgs,
    ExprFn: expr -> Expr,
    ExprCall: func -> Expr,
    ExprCall: args -> CallArgs,
    ExprLetIn: expr -> Expr,
    PatGrouped: pat  -> Pat,
    PatBinding: pat -> Pat,
    LetBinding: expr -> Expr,
}

define_multi_children! {
    ExprList: exprs -> Expr,
    ExprMap: pairs -> MapPair,
    ExprLetIn: bindings -> LetBinding,
    PatOr: pats -> Pat,
    PatList: pats -> Pat,
    FnArgs: exprs -> Expr,
    CallArgs: exprs -> Expr,
}

impl ExprBool {
    pub fn value(&self) -> Option<bool> {
        let token = self.nontrivial_tokens().next()?;
        Some(token.kind() == SyntaxKind::TokTrue)
    }
}

impl ExprInt {
    pub fn value(&self) -> Option<i64> {
        let token = self.nontrivial_tokens().next()?;
        parser::int_value(token.text())
    }
}

impl ExprFloat {
    pub fn value(&self) -> Option<f64> {
        let token = self.nontrivial_tokens().next()?;
        parser::float_value(token.text())
    }
}

impl ExprString {
    pub fn value(&self) -> Option<String> {
        let token = self.nontrivial_tokens().next()?;
        Some(parser::string_value(token.text()))
    }
}

impl ExprBinding {
    pub fn ident(&self) -> Option<Ident> {
        let token = self.nontrivial_tokens().next()?;
        Ident::cast(token)
    }
}

impl ExprBinary {
    pub fn op(&self) -> Option<SyntaxKind> {
        let token = self.nontrivial_tokens().next()?;
        Some(token.kind())
    }

    pub fn lhs(&self) -> Option<Expr> {
        self.syntax.first_child().and_then(Expr::cast)
    }

    pub fn rhs(&self) -> Option<Expr> {
        self.syntax.last_child().and_then(Expr::cast)
    }
}

impl ExprUnary {
    pub fn op(&self) -> Option<SyntaxKind> {
        let token = self.nontrivial_tokens().next()?;
        Some(token.kind())
    }
}

impl ExprIndex {
    pub fn is_nullable(&self) -> bool {
        matches!(
            self.nontrivial_tokens().next().map(|v| v.kind()),
            Some(SyntaxKind::TokQuestionDot | SyntaxKind::TokQuestionLBracket)
        )
    }

    pub fn is_shorthand(&self) -> bool {
        matches!(
            self.nontrivial_tokens().next().map(|v| v.kind()),
            Some(SyntaxKind::TokDot | SyntaxKind::TokQuestionDot)
        )
    }

    pub fn lhs(&self) -> Option<Expr> {
        self.syntax.first_child().and_then(Expr::cast)
    }

    pub fn rhs_expr(&self) -> Option<Expr> {
        self.syntax.last_child().and_then(Expr::cast)
    }

    pub fn rhs_ident(&self) -> Option<Ident> {
        let token = self.nontrivial_tokens().last()?;
        Ident::cast(token)
    }
}

impl ExprIfElse {
    pub fn cond(&self) -> Option<Expr> {
        self.syntax.first_child().and_then(Expr::cast)
    }

    pub fn if_true(&self) -> Option<Expr> {
        self.syntax.children().nth(1).and_then(Expr::cast)
    }

    pub fn if_false(&self) -> Option<Expr> {
        self.syntax.last_child().and_then(Expr::cast)
    }
}

impl MapPair {
    pub fn key_expr(&self) -> Option<Expr> {
        self.syntax.first_child().and_then(Expr::cast)
    }

    pub fn key_ident(&self) -> Option<Ident> {
        let token = self.nontrivial_tokens().next()?;
        Ident::cast(token)
    }

    pub fn value(&self) -> Option<Expr> {
        self.syntax.last_child().and_then(Expr::cast)
    }
}

impl LetBinding {
    pub fn ident(&self) -> Option<Ident> {
        let token = self.nontrivial_tokens().next()?;
        Ident::cast(token)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ident {
    syntax: SyntaxToken,
}

impl Ident {
    pub fn cast(syntax: SyntaxToken) -> Option<Ident> {
        if syntax.kind() == SyntaxKind::TokIdent {
            Some(Ident { syntax })
        } else {
            None
        }
    }

    pub fn syntax(&self) -> &SyntaxToken {
        &self.syntax
    }

    pub fn name(&self) -> &str {
        self.syntax.text()
    }
}
