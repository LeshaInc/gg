use std::hint::unreachable_unchecked;

use crate::syntax::BinOp::{self, *};
use crate::Type::{self, *};
use crate::Value;

macro_rules! as_int {
    ($val:expr) => {
        match $val.as_int() {
            Ok(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    };
}

macro_rules! add_ops {
    ($ctx:ident, $lut:ident { $(($lhs:expr, $rhs:expr, $op:expr) => |$x:ident, $y:ident| $func:expr,)* }) => {
        $(
        if $ctx.type_lhs == $lhs as usize
            && $ctx.type_rhs == $rhs as usize
            && $ctx.bin_op == $op as usize
        {
            #[inline(never)]
            fn operator($x: &Value, $y: &Value) ->  Value {
                ($func).into()
            }

            $lut[$lhs as usize][$rhs as usize][$op as usize] = operator;
        }
        )*
    };

    ($ctx:ident, $lut:ident) => {add_ops!($ctx, $lut {
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
    })};
}

type Operator = fn(&Value, &Value) -> Value;

const NUM_TYPES: usize = Type::VALUES.len();
const NUM_OPS: usize = BinOp::VALUES.len();

type Lut = [[[Operator; NUM_OPS]; NUM_TYPES]; NUM_TYPES];

fn bin_op_err(_: &Value, _: &Value) -> Value {
    panic!("invalid op")
}

struct Context {
    type_lhs: usize,
    type_rhs: usize,
    bin_op: usize,
}

const fn build_lut() -> Lut {
    let mut lut: Lut = [[[bin_op_err; NUM_OPS]; NUM_TYPES]; NUM_TYPES];

    let mut type_lhs = 0;
    while type_lhs < NUM_TYPES {
        let mut type_rhs = 0;
        while type_rhs < NUM_TYPES {
            let mut bin_op = 0;
            while bin_op < NUM_OPS {
                let ctx = Context {
                    type_lhs,
                    type_rhs,
                    bin_op,
                };

                add_ops!(ctx, lut);

                bin_op += 1;
            }

            type_rhs += 1;
        }

        type_lhs += 1;
    }

    lut
}

static LUT: Lut = build_lut();

pub fn bin_op(lhs: &Value, rhs: &Value, op: BinOp) -> Value {
    let func = LUT[lhs.ty() as usize][rhs.ty() as usize][op as usize];
    func(lhs, rhs)
}
