#![warn(clippy::identity_assign_op)]
#![allow(unused)]

const ONE: i64 = 1;
const ZERO: i64 = 0;

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

    x += 1; // no error
    x *= 2; // no error
    x -= 1; // no error
    x <<= 1; // no error
}
