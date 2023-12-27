#![warn(clippy::overflow_check_conditional)]
#![allow(clippy::needless_if)]

fn test(a: u32, b: u32, c: u32) {
    if a + b < a {}
    //~^ ERROR: you are trying to use classic C overflow conditions that will fail in Rust
    //~| NOTE: `-D clippy::overflow-check-conditional` implied by `-D warnings`
    if a > a + b {}
    //~^ ERROR: you are trying to use classic C overflow conditions that will fail in Rust
    if a + b < b {}
    //~^ ERROR: you are trying to use classic C overflow conditions that will fail in Rust
    if b > a + b {}
    //~^ ERROR: you are trying to use classic C overflow conditions that will fail in Rust
    if a - b > b {}
    //~^ ERROR: you are trying to use classic C underflow conditions that will fail in Rus
    if b < a - b {}
    //~^ ERROR: you are trying to use classic C underflow conditions that will fail in Rus
    if a - b > a {}
    //~^ ERROR: you are trying to use classic C underflow conditions that will fail in Rus
    if a < a - b {}
    //~^ ERROR: you are trying to use classic C underflow conditions that will fail in Rus
    if a + b < c {}
    if c > a + b {}
    if a - b < c {}
    if c > a - b {}
    let i = 1.1;
    let j = 2.2;
    if i + j < i {}
    if i - j < i {}
    if i > i + j {}
    if i - j < i {}
}

fn main() {
    test(1, 2, 3)
}
