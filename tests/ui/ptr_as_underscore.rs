#![warn(clippy::ptr_as_underscore)]
#![allow(dead_code)]
#![allow(unused)]

fn main() {
    let x = [3; 11];
    x.as_ptr() as *const _;
    let y = 1i32;
    let y = y as i64;
    // let 
}
