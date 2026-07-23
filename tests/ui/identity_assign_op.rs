#![warn(clippy::identity_assign_op)]
#![allow(unused)]

const ONE: i64 = 1;
const ZERO: i64 = 0;
const ONE_U64: u64 = 1;
const ZERO_U64: u64 = 0;
const ONE_F32: f32 = 1.0;
const ZERO_F32: f32 = 0.0;
const ONE_F64: f64 = 1.0;
const ZERO_F64: f64 = 0.0;

#[rustfmt::skip]
fn main() {
    let mut x = 1i64;

    x += 0;
    //~^ identity_assign_op

    x -= 0;
    //~^ identity_assign_op

    x |= 0;
    //~^ identity_assign_op

    x ^= 0;
    //~^ identity_assign_op

    x <<= 0;
    //~^ identity_assign_op

    x >>= 0;
    //~^ identity_assign_op

    x *= 1;
    //~^ identity_assign_op

    x /= 1;
    //~^ identity_assign_op

    x += ZERO;
    //~^ identity_assign_op

    x *= ONE;
    //~^ identity_assign_op

    let mut y = 1u64;

    y += 0u64;
    //~^ identity_assign_op

    y *= 1u64;
    //~^ identity_assign_op

    y += ZERO_U64;
    //~^ identity_assign_op

    y *= ONE_U64;
    //~^ identity_assign_op

    let mut z = 1.0f32;

    z += 0.0f32;
    //~^ identity_assign_op

    z *= 1.0f32;
    //~^ identity_assign_op

    z += ZERO_F32;
    //~^ identity_assign_op

    z *= ONE_F32;
    //~^ identity_assign_op

    let mut w = 1.0f64;

    w += 0.0;
    //~^ identity_assign_op

    w *= 1.0;
    //~^ identity_assign_op

    w += ZERO_F64;
    //~^ identity_assign_op

    w *= ONE_F64;
    //~^ identity_assign_op

    x += 1; // no error
    x *= 2; // no error
    x -= 1; // no error
    x <<= 1; // no error
}
