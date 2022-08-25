use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

use gg_expr::{compile, Source};

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let expr = gg_expr::syntax::parse(&input);
    let source = Arc::new(Source::new("unknown.expr".into(), input));

    println!("{:#?}", expr);

    let (value, diagnostics) = compile(source, expr);

    for diagnostic in diagnostics {
        println!("{}", diagnostic);
    }

    println!();
    println!("{:?}", value);
    println!();

    let t = Instant::now();
    if let Err(e) = value.force_eval() {
        println!("{}", e);
        return;
    }

    println!("{:?}", value);
    println!("took {:?}", t.elapsed());
}
