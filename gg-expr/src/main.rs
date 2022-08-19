use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

use gg_expr::syntax::Parser;
use gg_expr::{compile, Source};

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let source = Arc::new(Source::new("unknown.expr".into(), input));
    let mut parser = Parser::new(source);

    let expr = parser.expr();

    println!("{}", expr);

    for diagnostic in parser.diagnostics() {
        println!("{}", diagnostic);
    }

    let value = compile(&expr);

    println!();
    println!("{:?}", value);
    println!();

    let t = Instant::now();
    value.force_eval();

    println!("{:?}", value);
    println!("took {:?}", t.elapsed());
}
