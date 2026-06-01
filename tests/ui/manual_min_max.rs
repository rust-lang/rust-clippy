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

// Tie-breaking semantics test: S compares by `key` only; `payload` is ignored
// by Ord. When keys are equal, the two S values differ, so the choice of
// which is returned on a tie is observable.
#[derive(Eq, PartialEq)]
struct S {
    key: i32,
    payload: i32,
}
impl Ord for S {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}
impl PartialOrd for S {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn tie_breaking(a: S, b: S) {
    // When a.key == b.key, `if a < b { a } else { b }` returns b (else branch).
    // Correct suggestion must also return b on a tie: b.min(a).
    let _ = if a < b { a } else { b };
    //~^ manual_min_max
}

// Non-triggering: const context (Ord::min/max not const-stable)
const fn const_context(a: i32, b: i32) -> i32 {
    if a < b { a } else { b }
}
