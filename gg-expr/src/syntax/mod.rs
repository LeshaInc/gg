mod ast;
mod parser;
mod span;
mod token;

use miette::NamedSource;

pub use self::ast::*;
pub use self::parser::{Parser, SyntaxError};
pub use self::span::{Span, Spanned};
pub use self::token::{tokenize, Token};

pub fn report(source: &str, error: SyntaxError) {
    let error = miette::Report::from(error)
        .with_source_code(NamedSource::new("unknown.expr", String::from(source)));
    println!("{:?}", error);
}
