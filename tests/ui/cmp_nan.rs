#![warn(clippy::cmp_nan)]
#![allow(clippy::float_cmp, clippy::no_effect, clippy::unnecessary_operation)]

const NAN_F32: f32 = f32::NAN;
const NAN_F64: f64 = f64::NAN;

// don't lint these, since `{f32,f64}::is_nan` is not yet const stable
// FIXME: make this lint when `{f32,f64}::is_nan` becomes const stable
const IS_F32_NAN: bool = NAN_F32 == f32::NAN;
const IS_F64_NAN: bool = NAN_F64 == f64::NAN;

fn main() {
    let x = 5f32;
    x == f32::NAN;
    x != f32::NAN;
    x < f32::NAN;
    x > f32::NAN;
    x <= f32::NAN;
    x >= f32::NAN;
    x == NAN_F32;
    x != NAN_F32;
    x < NAN_F32;
    x > NAN_F32;
    x <= NAN_F32;
    x >= NAN_F32;

    let y = 0f64;
    y == f64::NAN;
    y != f64::NAN;
    y < f64::NAN;
    y > f64::NAN;
    y <= f64::NAN;
    y >= f64::NAN;
    y == NAN_F64;
    y != NAN_F64;
    y < NAN_F64;
    y > NAN_F64;
    y <= NAN_F64;
    y >= NAN_F64;
}
