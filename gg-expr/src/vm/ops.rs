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

macro_rules! as_list {
    ($val:expr) => {
        match $val.as_list() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! add_bin_ops {
    ($ctx:ident, $lut:ident) => {add_bin_ops!($ctx, $lut {
        (Int, Int, Add) => |x, y| as_int!(x) + as_int!(y),
        (Int, Int, Sub) => |x, y| as_int!(x) - as_int!(y),
        (Int, Int, Mul) => |x, y| as_int!(x) * as_int!(y),
        (Int, Int, Div) => |x, y| as_int!(x) as f64 / as_int!(y) as f64,

        (Int, Int, Lt) => |x, y| as_int!(x) < as_int!(y),
        (Int, Int, Le) => |x, y| as_int!(x) <= as_int!(y),
        (Int, Int, Eq) => |x, y| as_int!(x) == as_int!(y),
        (Int, Int, Neq) => |x, y| as_int!(x) != as_int!(y),
        (Int, Int, Ge) => |x, y| as_int!(x) >= as_int!(y),
        (Int, Int, Gt) => |x, y| as_int!(x) > as_int!(y),

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
    })};

    ($ctx:ident, $lut:ident { $(($lhs:expr, $rhs:expr, $op:expr) => |$x:ident, $y:ident| $func:expr,)* }) => {
        $(
        if $ctx.type_lhs == $lhs as usize
            && $ctx.type_rhs == $rhs as usize
            && $ctx.bin_op == $op as usize
        {
            #[inline(never)]
            fn operator($x: &Value, $y: &Value) -> Option<Value> {
                Some(($func).into())
            }

            $lut[$lhs as usize][$rhs as usize][$op as usize] = operator;
        }
        )*
    };
}

macro_rules! add_un_ops {
    ($ctx:ident, $lut:ident) => {add_un_ops!($ctx, $lut {
        (Int, Neg) => |x| -as_int!(x),
        (Int, Not) => |x| !as_int!(x),
    })};

    ($ctx:ident, $lut:ident { $(($val:expr, $op:expr) => |$x:ident| $func:expr,)* }) => {
        $(
        if $ctx.type_val == $val as usize
            && $ctx.un_op == $op as usize
        {
            #[inline(never)]
            fn operator($x: &Value) -> Option<Value> {
                Some(($func).into())
            }

            $lut[$val as usize][$op as usize] = operator;
        }
        )*
    };
}

type BinOpLut = [[[BinOpFn; NUM_BIN_OPS]; NUM_TYPES]; NUM_TYPES];

static BIN_OP_LUT: BinOpLut = build_bin_op_lut();

fn bin_op_err(_: &Value, _: &Value) -> Option<Value> {
    None
}

const fn build_bin_op_lut() -> BinOpLut {
    let mut lut: BinOpLut = [[[bin_op_err; NUM_BIN_OPS]; NUM_TYPES]; NUM_TYPES];

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

                add_bin_ops!(ctx, lut);

                bin_op += 1;
            }

            type_rhs += 1;
        }

        type_lhs += 1;
    }

    lut
}

pub fn bin_op(lhs: &Value, rhs: &Value, op: BinOp) -> Option<Value> {
    let func = BIN_OP_LUT[lhs.ty() as usize][rhs.ty() as usize][op as usize];
    func(lhs, rhs)
}

type UnOpLut = [[UnOpFn; NUM_UN_OPS]; NUM_TYPES];

static UN_OP_LUT: UnOpLut = build_un_op_lut();

fn un_op_err(_: &Value) -> Option<Value> {
    None
}

const fn build_un_op_lut() -> UnOpLut {
    let mut lut: UnOpLut = [[un_op_err; NUM_UN_OPS]; NUM_TYPES];

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

pub fn un_op(value: &Value, op: UnOp) -> Option<Value> {
    let func = UN_OP_LUT[value.ty() as usize][op as usize];
    func(value)
}
