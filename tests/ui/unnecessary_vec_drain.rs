#![allow(unused)]
#![warn(clippy::unnecessary_vec_drain)]

fn main() {
    let mut vec: Vec<i32> = Vec::new();
    //Lint
    vec.drain(..);
    vec.drain(0..vec.len());

    // Dont Lint
    let iter = vec.drain(..);
}
