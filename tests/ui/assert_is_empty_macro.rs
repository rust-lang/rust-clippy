//@no-rustfix: macro-expanded suggestions are intentionally help-only
#![warn(clippy::assert_is_empty)]

macro_rules! assert_empty {
    ($expr:expr) => {
        assert!($expr.is_empty());
        //~^ assert_is_empty
    };
}

macro_rules! assert_not_empty {
    ($expr:expr) => {
        assert!(!$expr.is_empty());
        //~^ assert_is_empty
    };
}

fn main() {
    let v: Vec<i32> = vec![];
    let s = String::new();

    assert_empty!(v);
    assert_not_empty!(s);
}
