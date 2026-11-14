#![warn(clippy::manual_vec)]
use std::collections::HashSet;

#[allow(clippy::useless_vec)]
fn main() {
    let _ = std::iter::repeat_n("abc", 3).collect::<Vec<_>>();
    //~^ manual_vec
    let _ = std::iter::repeat_n(some_elem(), some_count()).collect::<Vec<_>>();
    //~^ manual_vec
    let _ = std::iter::repeat_n("0451", some_count()).collect::<Vec<_>>();
    //~^ manual_vec
    let _: Vec<i32> = std::iter::repeat_n(some_elem(), 11).collect();
    //~^ manual_vec

    // not a vec - no lint
    let _: HashSet<i32> = std::iter::repeat_n(some_elem(), some_count()).collect();
    // intermediate iterator methods called - no lint
    let _: Vec<i32> = std::iter::repeat_n(some_elem(), some_count())
        .filter(|i| i.is_positive())
        .collect();
}

fn some_count() -> usize {
    19
}

fn some_elem() -> i32 {
    7274
}
