#![allow(clippy::needless_return, clippy::needless_else)]
#![warn(clippy::duplicate_match_guards)]

fn main() {
    let mut a = 5;
    let b = 4;
    let mut v: Vec<u32> = vec![];

    // Lint

    match 0u32 {
        0 if true => {
            if true {
                return;
            }
        },
        //~^^^^ duplicate_match_guards
        0 if a > b => {
            if a > b {
                return;
            }
        },
        //~^^^^ duplicate_match_guards

        // Not _identical_, but the meaning is the same
        0 if a > b => {
            if b < a {
                return;
            }
        },
        //~^^^^ duplicate_match_guards

        // A bit more complicated
        0 if a > 0 && b > 0 => {
            if a > 0 && b > 0 {
                return;
            }
        },
        //~^^^^ duplicate_match_guards

        // Comments inside the inner block are preserved
        // (for comments _outside_ the inner block, see below)
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {
                // before
                return;
                // after
            }
        },
        //~^^^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {// before, on the line of the second `if`
                return;
                // after
            }
        },
        //~^^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {/* before,
                continued */
                return;
                // after
            }
        },
        //~^^^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {
                // before
                return;
            /* after, on the line of the second `if` */}
        },
        //~^^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {// before, on the line of the second `if`
                return;
            /* after, on the line of the second `if` */}
        },
        //~^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {/* before,
                continued */
                return;
            /* after, on the line of the second `if` */}
        },
        //~^^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {
                // before
                return;
                /* after,
            continued */}
        },
        //~^^^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {// before, on the line of the second `if`
                return;
                /* after,
            continued*/}
        },
        //~^^^^^ duplicate_match_guards
        #[rustfmt::skip]
        0 if a > b => {
            if a > b {/* before,
                continued */
                return;
                /* after,
            continued*/}
        },
        //~^^^^^^ duplicate_match_guards

        // No curlies around arm body
        #[rustfmt::skip] // would add the outer curlies
        0 if true => if true {
            return;
        },
        //~^^^ duplicate_match_guards
        _ => unimplemented!(),
    }

    // Don't lint

    match 0 {
        0 if true => {
            if false {
                return;
            }
        },
        // Has side effects
        0 if v.pop().is_some() => {
            if v.pop().is_some() {
                return;
            }
        },
        // There's _something_ in the `else` branch
        0 if a > b => {
            if a > b {
                a += b;
            } else {
            }
        },
        // Body has statements before the `if`
        0 if a > b => {
            a += b;
            if a > b {
                return;
            }
        },
        // Comments (putting them right next to the tested element to check for off-by-one errors):
        // - before arrow
        #[rustfmt::skip]
        0 if a > b/* comment */=> {
            if a > b {
                return;
            }
        },
        // - before outer opening curly
        #[rustfmt::skip]
        0 if a > b =>/* comment */{
            if a > b {
                return;
            }
        },
        // - before `if`
        #[rustfmt::skip]
        0 if a > b => {/*
          comment
          */if a > b {
                return;
            }
        },
        // - before condition
        #[rustfmt::skip]
        0 if a > b => {
            if/*
            comment */a > b {
                return;
            }
        },
        #[rustfmt::skip]
        // - before inner opening curly
        0 if a > b => {
            if a > b/*
            comment */{
                return;
            }
        },
        #[rustfmt::skip]
        // - before closing outer curly
        0 if a > b => {
            if a > b {
                return;
            }/* comment
        */},
        _ => unimplemented!(),
    };
}
