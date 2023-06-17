//@aux-build:proc_macros.rs
#![allow(unused)]
#![warn(clippy::local_assigned_single_value)]

#[macro_use]
extern crate proc_macros;

fn a(a: &mut i32) {}

struct A(u32, u32);

#[repr(i32)]
enum B {
    A,
    B,
    C,
}

fn g(x: i32) -> i32 {
    x + 1
}

fn h() -> i32 {
    let mut x = 42;
    x = g(x);

    x
}

fn main() {
    let mut _a = A(1, 2);
    // Do not lint, unfortunately.
    let mut x = _a.0;
    x = 1;
    x = 1;
    // This lints, though!
    let mut x = (1,).0;
    x = 1;
    x = 1;
    let mut x = 1;
    x = 1;
    x = 1;
    // Do not lint
    x += 1;
    let mut x = 1;
    x = 1;
    x = 1;
    x = true as i32;
    x = B::B as i32;
    {
        x = 1;
    }
    let mut x = 1.0f32;
    x = 1.0;
    x = 1.0;
    {
        x = 1.0;
    }
    // Do not lint, unfortunately.
    let (mut x, y) = (1, 2);
    let [mut x, y] = [1, 2];
    let mut x = 1;
    x = 1;
    x = 1;
    // Don't lint
    a(&mut x);
    let mut x = 1;
    x = 1;
    x = 1;
    x = 1;
    x = 2;
    // Don't lint
    a(&mut x);
    let mut y = 1;
    // FIXME: Linting this requires nested dependencies, thus we don't currently, but please fix
    (x, y) = (1, 1);
    y = 1;
    // Don't lint
    let [mut x, y] = [1, 2];
    x = 1;
    x = 1;
    {
        x = 1;
    }
    let mut x = 1;
    x = 1;
    external! {
        let mut x = 1;
        x = 1;
        x = 1;
    }
    with_span! {
        span
        let mut x = 1;
        x = 1;
        x = 1;
    }
    let mut x = _a.1;
}
