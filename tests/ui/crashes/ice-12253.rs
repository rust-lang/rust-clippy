#[allow(overflowing_literals, unconditional_panic, clippy::no_effect, clippy::let_arr_const)]
fn main() {
    let arr: [i32; 5] = [0; 5];
    arr[0xfffffe7ffffffffffffffffffffffff];
}
