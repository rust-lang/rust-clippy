use core::num::Wrapping;

#[allow(dead_code, unused_assignments, clippy::useless_vec)]
#[warn(clippy::assign_op_pattern)]
fn main() {
    let mut a = 5;
    a = a + 1;
    //~^ ERROR: manual implementation of an assign operation
    //~| NOTE: `-D clippy::assign-op-pattern` implied by `-D warnings`
    a = 1 + a;
    //~^ ERROR: manual implementation of an assign operation
    a = a - 1;
    //~^ ERROR: manual implementation of an assign operation
    a = a * 99;
    //~^ ERROR: manual implementation of an assign operation
    a = 42 * a;
    //~^ ERROR: manual implementation of an assign operation
    a = a / 2;
    //~^ ERROR: manual implementation of an assign operation
    a = a % 5;
    //~^ ERROR: manual implementation of an assign operation
    a = a & 1;
    //~^ ERROR: manual implementation of an assign operation
    a = 1 - a;
    a = 5 / a;
    a = 42 % a;
    a = 6 << a;
    let mut s = String::new();
    s = s + "bla";
    //~^ ERROR: manual implementation of an assign operation

    // Issue #9180
    let mut a = Wrapping(0u32);
    a = a + Wrapping(1u32);
    //~^ ERROR: manual implementation of an assign operation
    let mut v = vec![0u32, 1u32];
    v[0] = v[0] + v[1];
    //~^ ERROR: manual implementation of an assign operation
    let mut v = vec![Wrapping(0u32), Wrapping(1u32)];
    v[0] = v[0] + v[1];
    let _ = || v[0] = v[0] + v[1];
}
