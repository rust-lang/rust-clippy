#![warn(clippy::suspicious_map)]
#![allow(clippy::redundant_closure)]

fn main() {
    let _ = (0..3).map(|x| x + 2).count();

    // This usage is OK because the `sum` side effect makes the `map` useful.
    let mut sum = 0;
    let _ = (0..3).map(|x| sum += x).count();

    // The linter is blind to path-based arguments however.
    let mut ext_sum = 0;
    let ext_closure = |x| ext_sum += x;
    let _ = (0..3).map(ext_closure).count();

    // The linter can see `FnMut` calls however.
    let mut ext_closure_inner = |x| ext_sum += x;
    let _ = (0..3).map(|x| ext_closure_inner(x)).count();
}
