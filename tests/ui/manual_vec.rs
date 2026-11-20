#![warn(clippy::manual_vec)]
#![expect(clippy::useless_vec)]

use std::collections::HashSet;
use std::iter::repeat_n;

fn main() {
    let _ = repeat_n("abc", 3).collect::<Vec<_>>();
    //~^ manual_vec
    let _ = repeat_n(black_box("hello!"), black_box(33)).collect::<Vec<_>>();
    //~^ manual_vec
    let _ = repeat_n("0451", black_box(39)).collect::<Vec<_>>();
    //~^ manual_vec
    let _: Vec<i32> = repeat_n(black_box(1), 11).collect();
    //~^ manual_vec

    // not a vec - no lint
    let _: HashSet<i32> = repeat_n(9, 31).collect();
    // intermediate iterator methods called - no lint
    let _: Vec<i32> = repeat_n(1, 2).filter(|i| i.is_positive()).collect();
}
