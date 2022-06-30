// run-rustfix

#![warn(clippy::bool_to_int_with_if)]

fn main() {
    let cond = true;

    // should warn
    let _x = if cond { 1 } else { 0 };

    let _x = if cond ^ true { 1 } else { 0 };

    // it should be type aware
    let _x: u8 = if cond { 1 } else { 0 };

    let _x = if cond { 1i16 } else { 0i16 };

    // shouldn't
    let _x = if cond { Some(1) } else { None };

    let _x = if cond {
        side_effect();
        1
    } else {
        0
    };

    let _x = if cond { 2 } else { 0 };

    // questionable
    // maybe `!cond as usize` is reasonable
    let _x = if cond { 0 } else { 1 };
}

fn side_effect() {}
