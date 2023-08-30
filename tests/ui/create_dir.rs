#![allow(unused_must_use)]
#![warn(clippy::create_dir)]

use std::fs::create_dir_all;

fn create_dir() {}

fn main() {
    // Should be warned
    std::fs::create_dir("foo");
    //~^ ERROR: calling `std::fs::create_dir` where there may be a better way
    //~| NOTE: `-D clippy::create-dir` implied by `-D warnings`
    std::fs::create_dir("bar").unwrap();
    //~^ ERROR: calling `std::fs::create_dir` where there may be a better way

    // Shouldn't be warned
    create_dir();
    std::fs::create_dir_all("foobar");
}
