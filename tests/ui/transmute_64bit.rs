//@ignore-32bit
#![allow(clippy::implicit_transmute_types)]

#[warn(clippy::wrong_transmute)]
fn main() {
    unsafe {
        let _: *const usize = std::mem::transmute(6.0f64);

        let _: *mut usize = std::mem::transmute(6.0f64);
    }
}
