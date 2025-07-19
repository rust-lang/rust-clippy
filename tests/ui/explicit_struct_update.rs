#![warn(clippy::explicit_struct_update)]

struct A {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
}

struct B;

struct C {
    a: i32,
    b: i32,
}

fn main() {
    // should not lint, no explicit struct update
    let a = A { a: 1, b: 2, c: 3, d: 4 };

    let b = A {
        a: a.a,
        b: a.b,
        c: a.c,
        d: a.d,
    };
    //~^^^^^^explicit_struct_update

    let c = A {
        a: a.a,
        b: a.b,
        c: 4,
        d: 5,
    };
    //~^^^^^^explicit_struct_update

    let d = A {
        a: a.a,
        b: a.b,
        c: a.c,
        d: 5,
    };
    //~^^^^^^explicit_struct_update

    // should not lint, only one field is updated
    let e = A {
        a: a.a,
        b: 5,
        c: 6,
        d: 7,
    };

    // should not lint, we already have update syntax
    let f = A { ..a };

    // should not lint, we already have update syntax
    let g = A { a: a.a, b: a.b, ..a };

    // should not lint, multiple bases
    let h = A {
        a: a.a,
        b: d.b,
        c: d.c,
        d: 5,
    };

    // should not lint, no fields
    let i = B {};

    // should not lint, no explicit struct update
    let j = C { a: 1, b: 2 };

    // should not lint, fields filled from different type
    let k = A {
        a: j.a,
        b: j.b,
        c: 3,
        d: 4,
    };
}
