#![allow(clippy::needless_if)]

fn main() {
    let x = 1;
    let y = 2;
    //~v double_comparisons
    if x == y || x < y {
        // do something
    }
    //~v double_comparisons
    if x < y || x == y {
        // do something
    }
    //~v double_comparisons
    if x == y || x > y {
        // do something
    }
    //~v double_comparisons
    if x > y || x == y {
        // do something
    }
    //~v double_comparisons
    if x < y || x > y {
        // do something
    }
    //~v double_comparisons
    if x > y || x < y {
        // do something
    }
    //~v double_comparisons
    if x <= y && x >= y {
        // do something
    }
    //~v double_comparisons
    if x >= y && x <= y {
        // do something
    }
}
