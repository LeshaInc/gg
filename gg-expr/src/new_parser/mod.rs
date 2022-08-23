mod lexer;
mod parser;
mod syntax_kind;

use rowan::SyntaxNode;

pub use self::lexer::Lexer;
pub use self::parser::Parser;
pub use self::syntax_kind::{ExprLang, SyntaxKind};

pub fn parse(source: &str) -> SyntaxNode<ExprLang> {
    let mut parser = Parser::new(source);
    parser.root();
    SyntaxNode::new_root(parser.finish())
}
