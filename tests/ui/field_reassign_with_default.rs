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

#[derive(Default)]
struct C {
    i: i32,
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
    let b = B { i: 42, j: 24 };

    // right
    let c: C = Default::default();
}
