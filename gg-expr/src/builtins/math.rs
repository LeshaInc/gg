use std::f32::consts;
use std::fmt::Display;

use crate::diagnostic::{Severity, SourceComponent};
use crate::{Error, ExtFunc, Map, Result, Value, VmContext};

fn any_error<E: Display>(ctx: &VmContext, idx: usize, error: E) -> Error {
    let ranges = ctx.cur_ranges();
    let call_range = ranges.as_ref().and_then(|v| v.get(0)).copied();
    let arg_range = ranges.as_ref().and_then(|v| v.get(2 + idx)).copied();
    let message = format!("{}", error);
    ctx.error(call_range, message, |diag, source| {
        if let (Some(source), Some(range)) = (source, arg_range) {
            diag.add_source(SourceComponent::new(source).with_label(Severity::Error, range, ""));
        }
    })
}

fn to_float(ctx: &VmContext, idx: usize, value: &Value) -> Result<f32> {
    value.as_float().map_err(|e| any_error(ctx, idx, e))
}

fn floor(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    if let Ok(x) = x.as_int() {
        return Ok(x.into());
    }

    let x = to_float(ctx, 0, x)?;
    Ok(x.floor().into())
}

fn ceil(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    if let Ok(x) = x.as_int() {
        return Ok(x.into());
    }

    let x = to_float(ctx, 0, x)?;
    Ok(x.ceil().into())
}

fn round(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    if let Ok(x) = x.as_int() {
        return Ok(x.into());
    }

    let x = to_float(ctx, 0, x)?;
    Ok(x.round().into())
}

fn abs(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    if let Ok(x) = x.as_int() {
        return Ok(x.abs().into());
    }

    let x = to_float(ctx, 0, x)?;
    Ok(x.abs().into())
}

fn trunc(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.trunc().into())
}

fn sin(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.sin().into())
}

fn cos(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.cos().into())
}

fn tan(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.tan().into())
}

fn sinh(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.sinh().into())
}

fn cosh(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.cosh().into())
}

fn tanh(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.tanh().into())
}

fn asin(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.asin().into())
}

fn acos(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.acos().into())
}

fn atan(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.atan().into())
}

fn asinh(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.asinh().into())
}

fn acosh(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.acosh().into())
}

fn atanh(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.atanh().into())
}

fn exp(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.exp().into())
}

fn ln(ctx: &VmContext, [x]: &[Value; 1]) -> Result<Value> {
    let x = to_float(ctx, 0, x)?;
    Ok(x.ln().into())
}

fn add_value(map: &mut Map, name: &str, val: impl Into<Value>) {
    map.insert(name.into(), val.into());
}

fn add_func<const N: usize, F>(map: &mut Map, name: &str, func: F)
where
    F: Fn(&VmContext, &[Value; N]) -> Result<Value> + 'static,
{
    add_value(map, name, ExtFunc::new(func));
}

pub fn module() -> Value {
    let mut map = Map::new();

    add_value(&mut map, "PI", consts::PI);
    add_value(&mut map, "TAU", consts::TAU);
    add_value(&mut map, "E", consts::E);
    add_value(&mut map, "EPSILON", f32::EPSILON);

    add_func(&mut map, "floor", floor);
    add_func(&mut map, "ceil", ceil);
    add_func(&mut map, "round", round);
    add_func(&mut map, "abs", abs);
    add_func(&mut map, "trunc", trunc);
    add_func(&mut map, "sin", sin);
    add_func(&mut map, "cos", cos);
    add_func(&mut map, "tan", tan);
    add_func(&mut map, "sinh", sinh);
    add_func(&mut map, "cosh", cosh);
    add_func(&mut map, "tanh", tanh);
    add_func(&mut map, "asin", asin);
    add_func(&mut map, "acos", acos);
    add_func(&mut map, "atan", atan);
    add_func(&mut map, "asinh", asinh);
    add_func(&mut map, "acosh", acosh);
    add_func(&mut map, "atanh", atanh);
    add_func(&mut map, "exp", exp);
    add_func(&mut map, "ln", ln);

    map.into()
}
