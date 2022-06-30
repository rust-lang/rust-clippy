// run-rustfix

#![warn(clippy::bool_to_int_with_if)]

fn main() {
    let cond = true;

    // should warn
    let x = if cond { 1 } else { 0 };

    let x = if cond ^ true { 1 } else { 0 };

    // it should be type aware
    let x: u8 = if cond { 1 } else { 0 };
    let x = if cond { 1i16 } else { 0i16 };

    // shouldn't

    let x = if cond { Some(1) } else { None };

    let x = if cond {
        side_effect();
        1
    } else {
        0
    };

    let x = if cond { 2 } else { 0 };

    // questionable
    // maybe `!cond as usize` is reasonable
    let x = if cond { 0 } else { 1 };
}

fn side_effect() {}
