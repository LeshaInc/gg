mod ast;
mod kind;
mod lexer;
mod op;
mod parser;
mod span;

pub use rowan::{TextRange, TextSize};

pub use self::ast::*;
pub use self::kind::{ExprLang, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
pub use self::lexer::Lexer;
pub use self::op::{BinOp, UnOp};
pub use self::parser::Parser;
pub use self::span::Spanned;

pub fn parse(source: &str) -> Expr {
    let mut parser = Parser::new(source);
    parser.root();
    let root = SyntaxNode::new_root(parser.finish());
    root.first_child().and_then(Expr::cast).unwrap()
}
