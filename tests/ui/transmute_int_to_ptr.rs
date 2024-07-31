#![warn(clippy::wrong_transmute)]

#[repr(transparent)]
#[allow(dead_code)]
pub struct K1 {
    b: usize,
    c: (),
}

pub fn foo(k: K1) {
    let x = unsafe { std::mem::transmute::<K1, *mut i8>(k) };
    std::hint::black_box(unsafe { *x });
}

#[allow(dead_code)]
pub struct K2 {
    b: usize,
    c: (),
}

pub fn bar(k: K2) {
    let x = unsafe { std::mem::transmute::<K2, *mut i8>(k) };
    std::hint::black_box(unsafe { *x });
}

fn main() {}
