use std::io::Read;

use gg_expr::{compile_text, Vm};

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();

    let (value, diagnostics) = compile_text(&input);

    for diagnostic in diagnostics {
        println!("{}", diagnostic);
    }

    let func = match value {
        Some(v) => v.try_into().unwrap(),
        None => return,
    };

    println!();
    println!("{:?}", func);

    let mut vm = Vm::new();
    let t = std::time::Instant::now();
    let result = vm.eval(func);
    let elapsed = t.elapsed();
    println!();
    println!("{:?}", result);
    println!("elapsed {:?}", elapsed);
}
