#![allow(unused)]
#![warn(clippy::transmute_slice_to_larger_element_type)]

fn i8_slice_to_i32_slice() {
    let i8_slice: &[i8] = &[1i8, 2, 3, 4];
    let i32_slice: &[i32] = unsafe { std::mem::transmute(i8_slice) };
}

fn main() {}
