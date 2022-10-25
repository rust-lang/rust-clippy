#![allow(unused)]
#![warn(clippy::mod_lib)]

mod lib;

fn main() {
    println!("{}", lib::something());
}
