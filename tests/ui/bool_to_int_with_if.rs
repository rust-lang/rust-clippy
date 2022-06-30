#![warn(clippy::bool_to_int_with_if)]

fn main() {
    // should warn
    let cond = true;
    let x = if (cond) { 1 } else { 0 };

    // shouldn't
    let x = if (cond) { Some(1) } else { None };

    let x = if (cond) {
        side_effect();
        1
    } else {
        0
    };

    let x = if (cond) { 2 } else { 0 };

    // questionable
    // maybe `!cond as usize` is reasonable
    let x = if (cond) { 0 } else { 1 };
}

fn side_effect() {}
