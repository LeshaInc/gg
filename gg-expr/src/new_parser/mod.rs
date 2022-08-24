mod ast;
mod lexer;
mod parser;
mod syntax_kind;

pub use rowan::{TextRange, TextSize};

pub use self::ast::*;
pub use self::lexer::Lexer;
pub use self::parser::Parser;
pub use self::syntax_kind::{ExprLang, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

pub fn parse(source: &str) -> SyntaxNode {
    let mut parser = Parser::new(source);
    parser.root();
    SyntaxNode::new_root(parser.finish())
}
