use criterion::{criterion_group, criterion_main, Criterion};
use gg_expr::{compile_text, Value, Vm};

fn fib(vm: &mut Vm, func: &Value, arg: i64) -> i64 {
    vm.eval(func, &[arg.into()]).as_int().unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut vm = Vm::new();
    let func = compile_text("let fib = fn(x): if x < 2 then x else fib(x - 2) + fib(x - 1) in fib");
    c.bench_function("fib 25", |b| b.iter(|| fib(&mut vm, &func, 25)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);