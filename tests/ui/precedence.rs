#![warn(clippy::precedence)]
#![allow(
    unused_must_use,
    clippy::no_effect,
    clippy::unnecessary_operation,
    clippy::clone_on_copy,
    clippy::identity_op,
    clippy::eq_op
)]

macro_rules! trip {
    ($a:expr) => {
        match $a & 0b1111_1111u8 {
            0 => println!("a is zero ({})", $a),
            _ => println!("a is {}", $a),
        }
    };
}

fn main() {
    1 << 2 + 3;
    //~^ precedence
    1 + 2 << 3;
    //~^ precedence
    4 >> 1 + 1;
    //~^ precedence
    1 + 3 >> 2;
    //~^ precedence
    1 ^ 1 - 1;
    //~^ precedence
    3 | 2 - 1;
    //~^ precedence
    3 & 5 - 2;
    //~^ precedence
    0x0F00 & 0x00F0 << 4;
    0x0F00 & 0xF000 >> 4;
    0x0F00 << 1 ^ 3;
    0x0F00 << 1 | 2;

    let b = 3;
    trip!(b * 8);
}

struct W(u8);
impl Clone for W {
    fn clone(&self) -> Self {
        W(1)
    }
}

fn closure_method_call() {
    // Do not lint when the method call is applied to the block, both inside the closure
    let f = |x: W| { x }.clone();
    assert!(matches!(f(W(0)), W(1)));

    let f = |x: W| -> _ { x }.clone();
    assert!(matches!(f(W(0)), W(0)));
    //~^^ precedence

    let f = move |x: W| -> _ { x }.clone();
    assert!(matches!(f(W(0)), W(0)));
    //~^^ precedence
}
