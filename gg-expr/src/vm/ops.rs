use std::hint::unreachable_unchecked;

use crate::syntax::BinOp::{self, *};
use crate::syntax::UnOp::{self, *};
use crate::Type::{self, *};
use crate::Value;

const NUM_TYPES: usize = Type::VALUES.len();
const NUM_BIN_OPS: usize = BinOp::VALUES.len();
const NUM_UN_OPS: usize = UnOp::VALUES.len();

type BinOpFn = fn(&Value, &Value) -> Option<Value>;
type UnOpFn = fn(&Value) -> Option<Value>;

macro_rules! as_int {
    ($val:expr) => {
        match $val.as_int() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! as_float {
    ($val:expr) => {
        match $val.as_float() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! as_bool {
    ($val:expr) => {
        match $val.as_bool() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! as_list {
    ($val:expr) => {
        match $val.as_list() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! as_map {
    ($val:expr) => {
        match $val.as_map() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! as_string {
    ($val:expr) => {
        match $val.as_string() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! add_bin_ops {
    ($ctx:ident, $lut:ident) => {add_bin_ops!($ctx, $lut {
        (Int, Int, Add) => |x, y| {
            let (x, y) = (as_int!(x), as_int!(y));
            x.checked_add(y).map(Value::from)
                .unwrap_or_else(|| ((x as f64) + (y as f64)).into())
        },
        (Int, Int, Sub) => |x, y| {
            let (x, y) = (as_int!(x), as_int!(y));
            x.checked_sub(y).map(Value::from)
                .unwrap_or_else(|| ((x as f64) - (y as f64)).into())
        },
        (Int, Int, Mul) => |x, y| {
            let (x, y) = (as_int!(x), as_int!(y));
            x.checked_mul(y).map(Value::from)
                .unwrap_or_else(|| ((x as f64) * (y as f64)).into())
        },
        (Int, Int, Div) => |x, y| {
            (as_int!(x) as f64) / (as_int!(y) as f64)
        },
        (Int, Int, Rem) => |x, y| {
            as_int!(x).wrapping_rem(as_int!(y))
        },
        (Int, Int, Pow) => |x, y| {
            let (x, y) = (as_int!(x), as_int!(y));
            if y > 0 {
                let y = y.min(i32::MAX.into()) as u32;
                x.checked_pow(y).map(Value::from)
                    .unwrap_or_else(|| ((x as f64).powi(y as i32)).into())
            } else {
                (x as f64).powf(y as f64).into()
            }
        },

        (Int, Float, Add) => |x, y| (as_int!(x) as f64) + as_float!(y),
        (Int, Float, Sub) => |x, y| (as_int!(x) as f64) - as_float!(y),
        (Int, Float, Mul) => |x, y| (as_int!(x) as f64) * as_float!(y),
        (Int, Float, Div) => |x, y| (as_int!(x) as f64) / as_float!(y),
        (Int, Float, Rem) => |x, y| (as_int!(x) as f64) % as_float!(y),
        (Int, Float, Pow) => |x, y| (as_int!(x) as f64).powf(as_float!(y)),

        (Float, Int, Add) => |x, y| as_float!(x) + (as_int!(y) as f64),
        (Float, Int, Sub) => |x, y| as_float!(x) - (as_int!(y) as f64),
        (Float, Int, Mul) => |x, y| as_float!(x) * (as_int!(y) as f64),
        (Float, Int, Div) => |x, y| as_float!(x) / (as_int!(y) as f64),
        (Float, Int, Rem) => |x, y| as_float!(x) % (as_int!(y) as f64),
        (Float, Int, Pow) => |x, y| as_float!(x).powf(as_int!(y) as f64),

        (Float, Float, Add) => |x, y| as_float!(x) + as_float!(y),
        (Float, Float, Sub) => |x, y| as_float!(x) - as_float!(y),
        (Float, Float, Mul) => |x, y| as_float!(x) * as_float!(y),
        (Float, Float, Div) => |x, y| as_float!(x) / as_float!(y),
        (Float, Float, Rem) => |x, y| as_float!(x) % as_float!(y),
        (Float, Float, Pow) => |x, y| as_float!(x).powf(as_float!(y)),

        (List, List, Add) => |x, y| {
            let mut x = as_list!(x).clone();
            x.append(as_list!(y).clone());
            x
        },

        (List, Int, Mul) => |x, y| {
            let x = as_list!(x);
            let mut res = im::Vector::new();
            for _ in 0..as_int!(y) {
                res.append(x.clone());
            }
            res
        },

        (List, Int, Index) => |x, y| {
            let index = usize::try_from(as_int!(y)).ok()?;
            as_list!(x).get(index)?.clone()
        },

        (List, Int, IndexNullable) => |x, y| {
            usize::try_from(as_int!(y)).ok()
                .and_then(|idx| as_list!(x).get(idx)?.clone().into())
                .unwrap_or(Value::null())
        },

        (String, String, Add) => |x, y| {
            let mut x = as_string!(x).to_owned();
            x.push_str(as_string!(y));
            x
        },

        (String, Int, Mul) => |x, y| {
            let y = as_int!(y);
            if y > 0 {
                as_string!(x).repeat(y as usize)
            } else {
                "".into()
            }
        },

        (Int, Int, Lt) => |x, y| as_int!(x) < as_int!(y),
        (Int, Int, Le) => |x, y| as_int!(x) <= as_int!(y),
        (Int, Int, Eq) => |x, y| as_int!(x) == as_int!(y),
        (Int, Int, Neq) => |x, y| as_int!(x) != as_int!(y),
        (Int, Int, Ge) => |x, y| as_int!(x) >= as_int!(y),
        (Int, Int, Gt) => |x, y| as_int!(x) > as_int!(y),

        (Float, Int, Lt) => |x, y| as_float!(x) < (as_int!(y) as f64),
        (Float, Int, Le) => |x, y| as_float!(x) <= (as_int!(y) as f64),
        (Float, Int, Eq) => |x, y| as_float!(x) == (as_int!(y) as f64),
        (Float, Int, Neq) => |x, y| as_float!(x) != (as_int!(y) as f64),
        (Float, Int, Ge) => |x, y| as_float!(x) >= (as_int!(y) as f64),
        (Float, Int, Gt) => |x, y| as_float!(x) > (as_int!(y) as f64),

        (Int, Float, Lt) => |x, y| (as_int!(x) as f64) < as_float!(y),
        (Int, Float, Le) => |x, y| (as_int!(x) as f64) <= as_float!(y),
        (Int, Float, Eq) => |x, y| (as_int!(x) as f64) == as_float!(y),
        (Int, Float, Neq) => |x, y| (as_int!(x) as f64) != as_float!(y),
        (Int, Float, Ge) => |x, y| (as_int!(x) as f64) >= as_float!(y),
        (Int, Float, Gt) => |x, y| (as_int!(x) as f64) > as_float!(y),

        (Float, Float, Lt) => |x, y| as_float!(x) < as_float!(y),
        (Float, Float, Le) => |x, y| as_float!(x) <= as_float!(y),
        (Float, Float, Eq) => |x, y| as_float!(x) == as_float!(y),
        (Float, Float, Neq) => |x, y| as_float!(x) != as_float!(y),
        (Float, Float, Ge) => |x, y| as_float!(x) >= as_float!(y),
        (Float, Float, Gt) => |x, y| as_float!(x) > as_float!(y),

        (String, String, Lt) => |x, y| as_string!(x) < as_string!(y),
        (String, String, Le) => |x, y| as_string!(x) <= as_string!(y),
        (String, String, Eq) => |x, y| as_string!(x) == as_string!(y),
        (String, String, Neq) => |x, y| as_string!(x) != as_string!(y),
        (String, String, Ge) => |x, y| as_string!(x) >= as_string!(y),
        (String, String, Gt) => |x, y| as_string!(x) > as_string!(y),

        (Bool, Bool, And) => |x, y| as_bool!(x) & as_bool!(y),
        (Bool, Bool, Or) => |x, y| as_bool!(x) | as_bool!(y),
        (Bool, Null, And) => |_, _| false,
        (Bool, Null, Or) => |x, _| as_bool!(x),
        (Null, Bool, And) => |_, _| false,
        (Null, Bool, Or) => |_, y| as_bool!(y),

        (Bool, Bool, Eq) => |x, y| as_bool!(x) == as_bool!(y),
        (String, String, Eq) => |x, y| as_string!(x) == as_string!(y),
        (List, List, Eq) => |x, y| as_list!(x) == as_list!(y),
    })};

    ($ctx:ident, $lut:ident { $(($lhs:expr, $rhs:expr, $op:expr) => |$x:pat_param, $y:pat_param| $func:expr,)* }) => {
        $(
        if $ctx.type_lhs == $lhs as usize
            && $ctx.type_rhs == $rhs as usize
            && $ctx.bin_op == $op as usize
        {
            fn operator($x: &Value, $y: &Value) -> Option<Value> {
                Some(($func).into())
            }

            $lut[$op as usize][$lhs as usize][$rhs as usize] = operator;
        }
        )*
    };
}

macro_rules! add_un_ops {
    ($ctx:ident, $lut:ident) => {add_un_ops!($ctx, $lut {
        (Int, Neg) => |x| -as_int!(x),
        (Int, Not) => |x| !as_int!(x),

        (Bool, Not) => |x| !as_bool!(x),
        (Null, Not) => |_| true,
    })};

    ($ctx:ident, $lut:ident { $(($val:expr, $op:expr) => |$x:pat_param| $func:expr,)* }) => {
        $(
        if $ctx.type_val == $val as usize
            && $ctx.un_op == $op as usize
        {
            fn operator($x: &Value) -> Option<Value> {
                Some(($func).into())
            }

            $lut[$op as usize][$val as usize] = operator;
        }
        )*
    };
}

type BinOpLut = [[[BinOpFn; NUM_TYPES]; NUM_TYPES]; NUM_BIN_OPS];

const BIN_OP_LUT: BinOpLut = build_bin_op_lut();

fn bin_op_err(_: &Value, _: &Value) -> Option<Value> {
    None
}

fn bin_op_eq(a: &Value, b: &Value) -> Option<Value> {
    Some((a == b).into())
}

fn bin_op_ret_a(a: &Value, _: &Value) -> Option<Value> {
    Some(a.clone())
}

fn bin_op_ret_b(_: &Value, b: &Value) -> Option<Value> {
    Some(b.clone())
}

fn bin_op_map_idx(map: &Value, idx: &Value) -> Option<Value> {
    as_map!(map).get(idx).cloned()
}

fn bin_op_map_idx_nullable(map: &Value, idx: &Value) -> Option<Value> {
    Some(as_map!(map).get(idx).cloned().unwrap_or_else(Value::null))
}

const fn build_bin_op_lut() -> BinOpLut {
    let mut lut: BinOpLut = [[[bin_op_err; NUM_TYPES]; NUM_TYPES]; NUM_BIN_OPS];

    let mut type_lhs = 0;
    while type_lhs < NUM_TYPES {
        let mut type_rhs = 0;
        while type_rhs < NUM_TYPES {
            let mut bin_op = 0;
            while bin_op < NUM_BIN_OPS {
                struct Context {
                    type_lhs: usize,
                    type_rhs: usize,
                    bin_op: usize,
                }

                let ctx = Context {
                    type_lhs,
                    type_rhs,
                    bin_op,
                };

                lut[bin_op][type_lhs][type_rhs] = if bin_op == Eq as usize {
                    bin_op_eq
                } else if bin_op == Coalesce as usize && type_lhs == Null as usize {
                    bin_op_ret_b
                } else if bin_op == Coalesce as usize {
                    bin_op_ret_a
                } else if bin_op == Index as usize && type_lhs == Map as usize {
                    bin_op_map_idx
                } else if bin_op == IndexNullable as usize && type_lhs == Map as usize {
                    bin_op_map_idx_nullable
                } else {
                    add_bin_ops!(ctx, lut);
                    bin_op += 1;
                    continue;
                };

                bin_op += 1;
            }

            type_rhs += 1;
        }

        type_lhs += 1;
    }

    lut
}

pub fn bin_op(op: BinOp, lhs: &Value, rhs: &Value) -> Option<Value> {
    let func = BIN_OP_LUT[op as usize][lhs.ty() as usize][rhs.ty() as usize];
    func(lhs, rhs)
}

type UnOpLut = [[UnOpFn; NUM_TYPES]; NUM_UN_OPS];

const UN_OP_LUT: UnOpLut = build_un_op_lut();

fn un_op_err(_: &Value) -> Option<Value> {
    None
}

const fn build_un_op_lut() -> UnOpLut {
    let mut lut: UnOpLut = [[un_op_err; NUM_TYPES]; NUM_UN_OPS];

    let mut type_val = 0;
    while type_val < NUM_TYPES {
        let mut un_op = 0;
        while un_op < NUM_UN_OPS {
            struct Context {
                type_val: usize,
                un_op: usize,
            }

            let ctx = Context { type_val, un_op };
            add_un_ops!(ctx, lut);

            un_op += 1;
        }

        type_val += 1;
    }

    lut
}

pub fn un_op(op: UnOp, value: &Value) -> Option<Value> {
    let func = UN_OP_LUT[op as usize][value.ty() as usize];
    func(value)
}
