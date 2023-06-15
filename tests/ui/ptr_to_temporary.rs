#![allow(clippy::explicit_auto_deref, clippy::unnecessary_cast, clippy::useless_vec)]

use std::ptr::addr_of;

fn a() -> i32 {
    0
}

struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

fn main() {
    let _p = &0 as *const i32;
    let _p = &a() as *const i32;
    let vec = vec![1];
    let _p = &vec.len() as *const usize;
    let x = &(1 + 2) as *const i32;
    let x = &(x as *const i32) as *const *const i32;

    // Do not lint...
    let ptr = &Vec3 { x: 1.0, y: 2.0, z: 3.0 };
    let some_variable = 1i32;
    let x = &(*ptr).x as *const f32;
    let x = &(some_variable) as *const i32;

    // ...As `addr_of!` does not report anything out of the ordinary
    let ptr = &Vec3 { x: 1.0, y: 2.0, z: 3.0 };
    let some_variable = 1i32;
    let x = addr_of!((*ptr).x) as *const f32;
    let x = addr_of!(some_variable);
}
