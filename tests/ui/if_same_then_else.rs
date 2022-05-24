#![warn(clippy::if_same_then_else)]
#![allow(
    clippy::blacklisted_name,
    clippy::eq_op,
    clippy::never_loop,
    clippy::no_effect,
    clippy::unused_unit,
    clippy::zero_divided_by_zero,
    clippy::branches_sharing_code
)]

struct Foo {
    bar: u8,
}

fn foo() -> bool {
    unimplemented!()
}

fn if_same_then_else() {
    if true {
        Foo { bar: 42 };
        0..10;
        ..;
        0..;
        ..10;
        0..=10;
        foo();
    } else {
        //~ ERROR same body as `if` block
        Foo { bar: 42 };
        0..10;
        ..;
        0..;
        ..10;
        0..=10;
        foo();
    }

    if true {
        Foo { bar: 42 };
    } else {
        Foo { bar: 43 };
    }

    if true {
        ();
    } else {
        ()
    }

    if true {
        0..10;
    } else {
        0..=10;
    }

    if true {
        foo();
        foo();
    } else {
        foo();
    }

    let _ = if true {
        0.0
    } else {
        //~ ERROR same body as `if` block
        0.0
    };

    let _ = if true {
        -0.0
    } else {
        //~ ERROR same body as `if` block
        -0.0
    };

    let _ = if true { 0.0 } else { -0.0 };

    // Different NaNs
    let _ = if true { 0.0 / 0.0 } else { f32::NAN };

    if true {
        foo();
    }

    let _ = if true {
        42
    } else {
        //~ ERROR same body as `if` block
        42
    };

    if true {
        let bar = if true { 42 } else { 43 };

        while foo() {
            break;
        }
        bar + 1;
    } else {
        //~ ERROR same body as `if` block
        let bar = if true { 42 } else { 43 };

        while foo() {
            break;
        }
        bar + 1;
    }

    if true {
        let _ = match 42 {
            42 => 1,
            a if a > 0 => 2,
            10..=15 => 3,
            _ => 4,
        };
    } else if false {
        foo();
    } else if foo() {
        let _ = match 42 {
            42 => 1,
            a if a > 0 => 2,
            10..=15 => 3,
            _ => 4,
        };
    }

    // Issue #8836
    if true { todo!() } else { todo!() }
}

// Issue #2423. This was causing an ICE.
fn func() {
    if true {
        f(&[0; 62]);
        f(&[0; 4]);
        f(&[0; 3]);
    } else {
        f(&[0; 62]);
        f(&[0; 6]);
        f(&[0; 6]);
    }
}

fn f(val: &[u8]) {}

mod issue_5698 {
    fn mul_not_always_commutative(x: i32, y: i32) -> i32 {
        if x == 42 {
            x * y
        } else if x == 21 {
            y * x
        } else {
            0
        }
    }
}

fn main() {}
