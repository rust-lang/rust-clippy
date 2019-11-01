#![warn(clippy::field_reassign_with_default)]

#[derive(Default)]
struct A {
    i: i32,
    j: i64,
}

struct B {
    i: i32,
    j: i64,
}

fn main() {
    // wrong
    let mut a: A = Default::default();
    a.i = 42;

    // right
    let mut a: A = Default::default();

    // right
    let a = A {
        i: 42,
        ..Default::default()
    };

    // right
    let mut a: A = Default::default();
    if a.i == 0 {
        a.j = 12;
    }

    // right
    let mut a: A = Default::default();
    let b = 5;

    // right
    let mut b = 32;
    let mut a: A = Default::default();
    b = 2;

    // right
    let b: B = B { i: 42, j: 24 };

    // right
    let mut b: B = B { i: 42, j: 24 };
    b.i = 52;

    // right
    let mut b = B { i: 15, j: 16 };
    let mut a: A = Default::default();
    b.i = 2;
}
