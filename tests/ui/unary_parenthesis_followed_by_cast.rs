#![allow(unused)]
#![allow(clippy::unnecessary_cast)]
#![warn(clippy::unary_parenthesis_followed_by_cast)]

fn hello(arg_1: f64) {}

fn main() {
    // fire
    let x = 3.0f32;

    hello((x) as f64);
}
