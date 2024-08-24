//@no-rustfix
//@aux-build:proc_macros.rs
#![deny(clippy::possible_missing_comma)]

use proc_macros::{external, inline_macros, with_span};

#[rustfmt::skip]
#[inline_macros]
fn main() {
    {
        let x = 20;
        let y = &32;

        let _ = [1, 2 *y]; //~ possible_missing_comma
        let _ = [1, 2 -x]; //~ possible_missing_comma
        let _ = [1, x - 2 -x]; //~ possible_missing_comma
        let _ = [1, 2 -x - 2]; //~ possible_missing_comma
        let _ = [1, 2 * x * 2 -x]; //~ possible_missing_comma

        let _ = [
            1,
            x
            -2 * x * 2 //~ possible_missing_comma
            -x, //~ possible_missing_comma
        ];

        let _ = (1 &x); //~ possible_missing_comma
        let _ = (1, x -1); //~ possible_missing_comma

        let _ = (1-x);
        let _ = (1- x);
        let _ = (1 -/* comment */x);
        let _ = (1, 2*y - 5);
        let _ = (1, 2 /x);
        let _ = (1, 2 <x);
        let _ = (1, 2 !=x);
        let _ = (1, 2 |x);
        let _ = (1, 2 <<x);

        let _ = [
            1
            - x,
            x
            * y,
        ];
    }

    {
        let x = true;
        let y = false;

        let _ = (y &&x); //~ possible_missing_comma
        let _ = (y && x);
        let _ = (y&&x);
    }

    with_span! {
        span
        let x = 0;
        let _ = (x -1);
    }

    external! {
        let x = 0;
        let _ = (x -1);
    }

    inline! {
        let x = 0;
        let _ = (x -1); //~ possible_missing_comma
        let _ = (x -$(@tt 1)); //~ possible_missing_comma
        let _ = ($(@expr x -1));
        let _ = (x $(@tt -)1);
    }
}
