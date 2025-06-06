#![warn(clippy::rest_when_destructuring_struct)]
#![allow(dead_code)]
#![allow(unused_variables)]

struct S {
    a: u8,
    b: u8,
    c: u8,
}

enum E {
    A { a1: u8, a2: u8 },
    B { b1: u8, b2: u8 },
}

fn main() {
    let s = S { a: 1, b: 2, c: 3 };

    let S { a, b, .. } = s;
    //~^ rest_when_destructuring_struct

    let e = E::A { a1: 1, a2: 2 };

    match e {
        E::A { a1, a2 } => (),
        E::B { .. } => (),
        //~^ rest_when_destructuring_struct
    }

    match e {
        E::A { a1: _, a2: _ } => (),
        E::B { b1, b2: _ } => (),
    }
}
