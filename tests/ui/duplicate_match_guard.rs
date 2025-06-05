#![allow(clippy::needless_return, clippy::needless_else)]
#![warn(clippy::duplicate_match_guard)]

fn main() {
    let mut a = 5;
    let b = 4;
    let mut v: Vec<u32> = vec![];

    match 0u32 {
        0 if true => {
            if true {
                //~^ duplicate_match_guard
                return;
            }
        },
        0 if a > b => {
            if a > b {
                //~^ duplicate_match_guard
                return;
            }
        },
        // not _identical_, but the meaning is the same
        0 if a > b => {
            if b < a {
                //~^ duplicate_match_guard
                return;
            }
        },
        // a bit more complicated
        0 if a > 0 && b > 0 => {
            if a > 0 && b > 0 {
                //~^ duplicate_match_guard
                return;
            }
        },

        // no warnings
        0 if true => {
            if false {
                return;
            }
        },
        // has side effects, so don't lint
        0 if v.pop().is_some() => {
            if v.pop().is_some() {
                return;
            }
        },
        // there's _something_ in the else branch
        0 if a > b => {
            if a > b {
                a += b;
            } else {
            }
        },
        // body has statements before the if
        0 if a > b => {
            a += b;
            if a > b {
                return;
            }
        },
        _ => todo!(),
    };
}
