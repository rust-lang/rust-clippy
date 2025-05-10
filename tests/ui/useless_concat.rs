//@aux-build:proc_macros.rs

#![warn(clippy::useless_concat)]
#![allow(clippy::print_literal)]

extern crate proc_macros;
use proc_macros::{external, with_span};

macro_rules! my_concat {
    ($fmt:literal $(, $e:expr)*) => {
        println!(concat!("ERROR: ", $fmt), $($e,)*);
    }
}

fn main() {
    let x = concat!(); //~ useless_concat
    let x = concat!("a"); //~ useless_concat
    let x = concat!(r##"a"##); //~ useless_concat
    let x = concat!(1); //~ useless_concat
    println!("b: {}", concat!("a")); //~ useless_concat
    // Should not lint.
    let x = concat!("a", "b");
    let local_i32 = 1;
    my_concat!("{}", local_i32);
    let x = concat!(file!(), "#L", line!());

    external! { concat!(); }
    with_span! {
        span
        concat!();
    }
}
