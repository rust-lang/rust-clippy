// run-rustfix

#![warn(clippy::as_underscore)]

fn foo(_n: usize) {}

fn use_ptr(_: *const *const ()) {}

#[allow(dead_code)]
struct UwU;

impl UwU {
    #[allow(dead_code)]
    fn as_ptr(&self) -> *const u8 {
        &self as *const _ as *const u8
    }
}

fn main() {
    let n: u16 = 256;
    foo(n as _);

    let n = 0_u128;
    let _n: u8 = n as _;

    let x = 1 as *const ();
    use_ptr(x as *const _);
    let x2 = x as *const _;
    use_ptr(x2);

    let _x3: *mut i32 = x as *mut _;

    let _x4: *const () = x as _;
}
