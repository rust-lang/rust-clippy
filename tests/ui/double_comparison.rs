#![allow(clippy::needless_if)]

fn main() {
    let x = 1;
    let y = 2;
    if x == y || x < y {
    //~^ ERROR: this binary expression can be simplified
    //~| NOTE: `-D clippy::double-comparisons` implied by `-D warnings`
        // do something
    }
    if x < y || x == y {
    //~^ ERROR: this binary expression can be simplified
        // do something
    }
    if x == y || x > y {
    //~^ ERROR: this binary expression can be simplified
        // do something
    }
    if x > y || x == y {
    //~^ ERROR: this binary expression can be simplified
        // do something
    }
    if x < y || x > y {
    //~^ ERROR: this binary expression can be simplified
        // do something
    }
    if x > y || x < y {
    //~^ ERROR: this binary expression can be simplified
        // do something
    }
    if x <= y && x >= y {
    //~^ ERROR: this binary expression can be simplified
        // do something
    }
    if x >= y && x <= y {
    //~^ ERROR: this binary expression can be simplified
        // do something
    }
}
