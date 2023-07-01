//@run-rustfix
//@aux-build:proc_macros.rs:proc-macro
#![allow(
    arithmetic_overflow,
    clippy::erasing_op,
    clippy::identity_op,
    clippy::no_effect,
    unused
)]
#![warn(clippy::inefficient_pow)]

#[macro_use]
extern crate proc_macros;

fn main() {
    _ = 2i32.pow(1);
    _ = 2i32.pow(0);
    _ = 2i32.pow(3);
    _ = 2i32.pow(100);
    _ = 4i32.pow(3);
    _ = 32i32.pow(3);
    _ = 64i32.pow(3);
    let n = 3;
    _ = 64i32.pow(n);
    // Overflows, so wrong return value, but that's fine. `arithmetic_overflow` will deny this anyway
    _ = 4096i32.pow(3);
    _ = 65536i32.pow(3);
    // Don't lint
    _ = 0i32.pow(3);
    _ = 1i32.pow(3);
    external! {
        _ = 2i32.pow(1);
        _ = 2i32.pow(0);
        _ = 2i32.pow(3);
        _ = 2i32.pow(100);
        _ = 4i32.pow(3);
        _ = 32i32.pow(3);
        _ = 64i32.pow(3);
        _ = 4096i32.pow(3);
        _ = 65536i32.pow(3);
    }
    with_span! {
        span
        _ = 2i32.pow(1);
        _ = 2i32.pow(0);
        _ = 2i32.pow(3);
        _ = 2i32.pow(100);
        _ = 4i32.pow(3);
        _ = 32i32.pow(3);
        _ = 64i32.pow(3);
        _ = 4096i32.pow(3);
        _ = 65536i32.pow(3);
    }
}
