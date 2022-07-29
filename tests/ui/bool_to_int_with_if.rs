// run-rustfix

#![warn(clippy::bool_to_int_with_if)]
#![allow(unused, dead_code, clippy::unnecessary_operation, clippy::no_effect)]

fn main() {
    let a = true;
    let b = false;

    let x = 1;
    let y = 2;

    // Should lint
    // precedence
    if a {
        1
    } else {
        0
    };
    if !a {
        1
    } else {
        0
    };
    if a || b {
        1
    } else {
        0
    };
    if cond(a, b) {
        1
    } else {
        0
    };
    if x + y < 4 {
        1
    } else {
        0
    };

    // if else if
    if a {
        123
    } else if b {
        1
    } else {
        0
    };

    // Shouldn't lint

    if a {
        1
    } else if b {
        0
    } else {
        3
    };

    if a {
        3
    } else if b {
        1
    } else {
        -2
    };

    if a {
        3
    } else {
        0
    };
    if a {
        side_effect();
        1
    } else {
        0
    };
    if a {
        1
    } else {
        side_effect();
        0
    };

    // multiple else ifs
    if a {
        123
    } else if b {
        1
    } else if a | b {
        0
    } else {
        123
    };

    some_fn(a);
}

// Lint returns and type inference
fn some_fn(a: bool) -> u8 {
    if a { 1 } else { 0 }
}

fn side_effect() {}

fn cond(a: bool, b: bool) -> bool {
    a || b
}
