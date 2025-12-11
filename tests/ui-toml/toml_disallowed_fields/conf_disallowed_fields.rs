#![warn(clippy::disallowed_fields)]

use std::ops::{Range, RangeTo};

struct X {
    y: u32,
}

use crate::X as Y;

fn main() {
    let x = X { y: 0 };
    let _ = x.y;
    //~^ disallowed_fields

    let x = Y { y: 0 };
    let _ = x.y;
    //~^ disallowed_fields

    let x = Range { start: 0, end: 0 };
    let _ = x.start;
    //~^ disallowed_fields
    let _ = x.end;
    //~^ disallowed_fields

    let x = RangeTo { end: 0 };
    let _ = x.end;
    //~^ disallowed_fields
}
