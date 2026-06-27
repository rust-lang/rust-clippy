#![warn(clippy::manual_min)]
#![allow(clippy::manual_max, clippy::needless_late_init, unused_assignments)]

fn main() {
    let (a, b) = (1, 2);

    // Value-returning `if`/`else`, both comparison directions.
    let _ = if a > b { b } else { a };
    //~^ manual_min
    let _ = if a >= b { b } else { a };
    //~^ manual_min
    let _ = if a < b { a } else { b };
    //~^ manual_min
    let _ = if a <= b { a } else { b };
    //~^ manual_min

    // Guarding `if` that lowers a value to a ceiling.
    let mut cores = a;
    if cores > b {
        cores = b;
    }
    //~^^^ manual_min

    let mut limit = a;
    if limit >= b {
        limit = b;
    }
    //~^^^ manual_min

    // Negatives: not a manual min.
    let _ = if a > b { a } else { b }; // this is a max
    let f = 1.0_f64;
    let g = 2.0_f64;
    let _ = if f > g { g } else { f }; // float: NaN semantics differ, no lint
}
