mod ast;
mod kind;
mod lexer;
mod parser;
mod span;

pub use rowan::{TextRange, TextSize};

pub use self::ast::*;
pub use self::kind::{ExprLang, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};
pub use self::lexer::Lexer;
pub use self::parser::{ParseResult, Parser};
pub use self::span::Spanned;

pub fn parse(source: &str) -> ParseResult {
    let mut parser = Parser::new(source);
    parser.root();
    parser.finish()
}
