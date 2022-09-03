use std::io::Read;

use gg_expr::compile_text;

fn main() {
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
}
