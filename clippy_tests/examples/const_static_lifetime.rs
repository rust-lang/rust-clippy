#![feature(plugin)]
#![plugin(clippy)]

const VAR_ONE: &'static str = "Test constant #1"; // ERROR Consider removing 'static.

const VAR_TWO: &str = "Test constant #2"; // This line should not raise a warning.

fn main() {

    println!("{}", VAR_ONE);
    println!("{}", VAR_TWO);

}
