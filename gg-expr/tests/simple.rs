use gg_expr::{eval, ExtFunc, Value, Vm};

fn check(code: &str, expected: impl Into<Value>) {
    let (res, diagnostics) = eval(code);
    assert!(diagnostics.is_empty());
    assert_eq!(res.unwrap(), expected.into());
}

fn check_func(code: &str, args: &[&Value], expected: impl Into<Value>) {
    let (func, diagnostics) = eval(code);
    let func = func.unwrap();
    assert!(diagnostics.is_empty());
    let mut vm = Vm::new();
    let res = vm.eval(&func, args);
    assert_eq!(res.unwrap(), expected.into());
}

#[test]
fn test_math() {
    check("1 + 2 * 3", 7);
}

#[test]
fn test_ext_func() {
    let func = Value::from(ExtFunc::new(|[x]| Value::from(x.as_int().unwrap() * 2)));
    check_func("fn(foo): foo(10)", &[&func], 20);
}
