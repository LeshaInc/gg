use std::io::Read;
use std::time::Instant;

use gg_expr::{compile_text, new_compiler, syntax};

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let res = syntax::parse(&input);
    for error in res.errors {
        println!("{}", error);
    }

    let expr = match res.expr {
        Some(v) => v,
        None => return,
    };

    new_compiler::compile(expr);
}

#[allow(dead_code)]
fn old_main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let (value, diagnostics) = compile_text(&input);

    for diagnostic in diagnostics {
        println!("{}", diagnostic);
    }

    let value = match value {
        Some(v) => v,
        None => return,
    };

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
