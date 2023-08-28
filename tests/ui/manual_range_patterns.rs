#![allow(unused)]
#![warn(clippy::manual_range_patterns)]
#![feature(exclusive_range_pattern)]

fn main() {
    let f = 6;

    let _ = matches!(f, 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10);
    //~^ ERROR: this OR pattern can be rewritten using a range
    //~| NOTE: `-D clippy::manual-range-patterns` implied by `-D warnings`
    let _ = matches!(f, 4 | 2 | 3 | 1 | 5 | 6 | 9 | 7 | 8 | 10);
    //~^ ERROR: this OR pattern can be rewritten using a range
    let _ = matches!(f, 4 | 2 | 3 | 1 | 5 | 6 | 9 | 8 | 10); // 7 is missing
    let _ = matches!(f, | 4);
    let _ = matches!(f, 4 | 5);
    let _ = matches!(f, 1 | 2147483647);
    let _ = matches!(f, 0 | 2147483647);
    let _ = matches!(f, -2147483647 | 2147483647);
    let _ = matches!(f, 1 | (2..=4));
    let _ = matches!(f, 1 | (2..4));
    let _ = matches!(f, (1..=10) | (2..=13) | (14..=48324728) | 48324729);
    //~^ ERROR: this OR pattern can be rewritten using a range
    let _ = matches!(f, 0 | (1..=10) | 48324730 | (2..=13) | (14..=48324728) | 48324729);
    //~^ ERROR: this OR pattern can be rewritten using a range
    let _ = matches!(f, 0..=1 | 0..=2 | 0..=3);
    //~^ ERROR: this OR pattern can be rewritten using a range
    #[allow(clippy::match_like_matches_macro)]
    let _ = match f {
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 => true,
        //~^ ERROR: this OR pattern can be rewritten using a range
        _ => false,
    };
    let _ = matches!(f, -1 | -5 | 3 | -2 | -4 | -3 | 0 | 1 | 2);
    //~^ ERROR: this OR pattern can be rewritten using a range
    let _ = matches!(f, -1 | -5 | 3 | -2 | -4 | -3 | 0 | 1); // 2 is missing
    let _ = matches!(f, -1_000_000..=1_000_000 | -1_000_001 | 1_000_001);
    //~^ ERROR: this OR pattern can be rewritten using a range
    let _ = matches!(f, -1_000_000..=1_000_000 | -1_000_001 | 1_000_002);

    macro_rules! mac {
        ($e:expr) => {
            matches!($e, 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10)
        };
    }
    mac!(f);
}
