use criterion::{criterion_group, criterion_main, Criterion};
use gg_expr::{compile_text, Value, Vm};

fn fib(vm: &mut Vm, func: &Value, arg: i32) -> i32 {
    vm.eval(func, &[func, &arg.into()])
        .unwrap()
        .as_int()
        .unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut vm = Vm::new();
    let source =
        "let fib = fn(fib, x): if x < 2 then x else fib(fib, x - 2) + fib(fib, x - 1) in fib";
    let (func, diags) = compile_text(source);
    assert!(diags.is_empty());
    let func = vm.eval(&func.unwrap(), &[]).unwrap();
    c.bench_function("fib 25", |b| b.iter(|| fib(&mut vm, &func, 25)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
