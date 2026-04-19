#![warn(clippy::manual_min_max)]
#![allow(unused, clippy::if_same_then_else)]

fn main() {
    let a: i32 = 3;
    let b: i32 = 7;

    // op=< → min
    let _ = if a < b { a } else { b };
    //~^ manual_min_max
    // op=<= → min
    let _ = if a <= b { a } else { b };
    //~^ manual_min_max
    // op=> (then=rhs) → min
    let _ = if a > b { b } else { a };
    //~^ manual_min_max
    // op=>= (then=rhs) → min
    let _ = if a >= b { b } else { a };
    //~^ manual_min_max

    // op=> (then=lhs) → max
    let _ = if a > b { a } else { b };
    //~^ manual_min_max
    // op=>= (then=lhs) → max
    let _ = if a >= b { a } else { b };
    //~^ manual_min_max
    // op=< (then=rhs) → max
    let _ = if a < b { b } else { a };
    //~^ manual_min_max
    // op=<= (then=rhs) → max
    let _ = if a <= b { b } else { a };
    //~^ manual_min_max

    // compound expression: receiver needs parenthesization
    let _ = if a + 1 < b { a + 1 } else { b };
    //~^ manual_min_max

    // Non-triggering: floats implement PartialOrd but not Ord
    let fa: f64 = 1.0;
    let fb: f64 = 2.0;
    let _ = if fa < fb { fa } else { fb };

    // Non-triggering: side effects (eq_expr_value rejects these)
    fn get() -> i32 {
        42
    }
    let _ = if get() < b { get() } else { b };

    // Non-triggering: mismatched branches
    let _ = if a < b { b } else { b };

    // Non-triggering: no else branch
    if a < b {
        let _ = a;
    }
}

// Non-triggering: const context (Ord::min/max not const-stable)
const fn const_context(a: i32, b: i32) -> i32 {
    if a < b { a } else { b }
}
