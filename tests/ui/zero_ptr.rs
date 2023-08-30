pub fn foo(_const: *const f32, _mut: *mut i64) {}

fn main() {
    let _ = 0 as *const usize;
    //~^ ERROR: `0 as *const _` detected
    //~| NOTE: `-D clippy::zero-ptr` implied by `-D warnings`
    let _ = 0 as *mut f64;
    //~^ ERROR: `0 as *mut _` detected
    let _: *const u8 = 0 as *const _;
    //~^ ERROR: `0 as *const _` detected

    foo(0 as _, 0 as _);
    foo(0 as *const _, 0 as *mut _);
    //~^ ERROR: `0 as *const _` detected
    //~| ERROR: `0 as *mut _` detected

    let z = 0;
    let _ = z as *const usize; // this is currently not caught
}
