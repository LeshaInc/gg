use std::fmt;
use std::hash::{Hash, Hasher};

use rowan::NodeOrToken;

use super::{parser, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange};
use crate::syntax::{BinOp, UnOp};

type WalkEvent = rowan::WalkEvent<NodeOrToken<SyntaxNode, SyntaxToken>>;

fn rev_preorder(node: &SyntaxNode) -> impl Iterator<Item = WalkEvent> {
    let root: NodeOrToken<SyntaxNode, SyntaxToken> = NodeOrToken::Node(node.clone());
    std::iter::successors(Some(WalkEvent::Enter(root.clone())), move |pos| {
        let next = match pos {
            WalkEvent::Enter(el) => match el {
                NodeOrToken::Node(node) => match node.last_child_or_token() {
                    Some(child) => WalkEvent::Enter(child),
                    None => WalkEvent::Leave(node.clone().into()),
                },
                NodeOrToken::Token(token) => WalkEvent::Leave(token.clone().into()),
            },
            WalkEvent::Leave(el) => {
                if el == &root {
                    return None;
                }

                match el.prev_sibling_or_token() {
                    Some(sibling) => WalkEvent::Enter(sibling),
                    None => WalkEvent::Leave(el.parent().unwrap().into()),
                }
            }
        };
        Some(next)
    })
}

fn first_non_trivial_token_range(it: impl Iterator<Item = WalkEvent>) -> Option<TextRange> {
    it.flat_map(|el| {
        if let WalkEvent::Enter(NodeOrToken::Token(tok)) = el {
            if !tok.kind().is_trivia() {
                return Some(tok.text_range());
            }
        }

        None
    })
    .next()
}

fn non_trivial_text_range(root: &SyntaxNode) -> TextRange {
    let start = first_non_trivial_token_range(root.preorder_with_tokens());
    let end = first_non_trivial_token_range(rev_preorder(root));

    if let (Some(start), Some(end)) = (start, end) {
        start.cover(end)
    } else {
        root.text_range()
    }
}

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

            pub fn range(&self) -> TextRange {
                non_trivial_text_range(&self.syntax)
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

            pub fn range(&self) -> TextRange {
                non_trivial_text_range(self.syntax())
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
    ExprCall,
    ExprIndex,
    ExprIfElse,
    ExprLetIn,
    ExprWhen,
    ExprFn,
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
    WhenCase,
];

define_enum!(Expr {
    Null(ExprNull),
    Bool(ExprBool),
    Int(ExprInt),
    Float(ExprFloat),
    String(ExprString),
    Binding(ExprBinding),
    Binary(ExprBinary),
    Unary(ExprUnary),
    Grouped(ExprGrouped),
    List(ExprList),
    Map(ExprMap),
    Call(ExprCall),
    Index(ExprIndex),
    IfElse(ExprIfElse),
    LetIn(ExprLetIn),
    When(ExprWhen),
    Fn(ExprFn),
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
    ExprLetIn: expr -> Expr,
    ExprWhen: expr -> Expr,
    ExprFn: expr -> Expr,
    PatGrouped: pat  -> Pat,
    PatBinding: pat -> Pat,
    LetBinding: expr -> Expr,
    WhenCase: pat -> Pat,
    WhenCase: expr -> Expr,
}

define_multi_children! {
    ExprList: exprs -> Expr,
    ExprMap: pairs -> MapPair,
    ExprLetIn: bindings -> LetBinding,
    ExprWhen: cases -> WhenCase,
    PatOr: pats -> Pat,
    PatList: pats -> Pat,
}

impl ExprBool {
    pub fn value(&self) -> Option<bool> {
        let token = self.nontrivial_tokens().next()?;
        Some(token.kind() == SyntaxKind::TokTrue)
    }
}

impl ExprInt {
    pub fn value(&self) -> Option<i32> {
        let token = self.nontrivial_tokens().next()?;
        parser::int_value(token.text())
    }
}

impl ExprFloat {
    pub fn value(&self) -> Option<f32> {
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
    pub fn op(&self) -> Option<BinOp> {
        let token = self.nontrivial_tokens().next()?;
        BinOp::from_token(token.kind())
    }

    pub fn lhs(&self) -> Option<Expr> {
        self.syntax.first_child().and_then(Expr::cast)
    }

    pub fn rhs(&self) -> Option<Expr> {
        self.syntax.last_child().and_then(Expr::cast)
    }
}

impl ExprUnary {
    pub fn op(&self) -> Option<UnOp> {
        let token = self.nontrivial_tokens().next()?;
        UnOp::from_token(token.kind())
    }
}

impl ExprIndex {
    pub fn op(&self) -> Option<BinOp> {
        let token = self.nontrivial_tokens().next().map(|v| v.kind())?;
        BinOp::from_token(token)
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

impl ExprCall {
    pub fn func(&self) -> Option<Expr> {
        self.syntax.first_child().and_then(Expr::cast)
    }

    pub fn args(&self) -> impl Iterator<Item = Expr> {
        self.syntax.children().skip(1).flat_map(Expr::cast)
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
        if self.key_ident().is_some() {
            return None;
        }

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

impl ExprFn {
    pub fn args(&self) -> impl Iterator<Item = Ident> {
        self.nontrivial_tokens().flat_map(Ident::cast)
    }
}

impl PatInt {
    pub fn value(&self) -> Option<i32> {
        let token = self.nontrivial_tokens().next()?;
        parser::int_value(token.text())
    }
}

impl PatString {
    pub fn value(&self) -> Option<String> {
        let token = self.nontrivial_tokens().next()?;
        Some(parser::string_value(token.text()))
    }
}

impl PatBinding {
    pub fn ident(&self) -> Option<Ident> {
        let token = self.nontrivial_tokens().last()?;
        Ident::cast(token)
    }
}

#[derive(Clone, Debug)]
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

    pub fn range(&self) -> TextRange {
        self.syntax.text_range()
    }

    pub fn name(&self) -> &str {
        self.syntax.text()
    }
}

impl PartialEq for Ident {
    fn eq(&self, rhs: &Ident) -> bool {
        self.name() == rhs.name()
    }
}

impl Eq for Ident {}

impl Hash for Ident {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state)
    }
}
