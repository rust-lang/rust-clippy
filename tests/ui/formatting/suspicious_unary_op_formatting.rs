//@aux-build:../auxiliary/proc_macros.rs
#![deny(clippy::suspicious_unary_op_formatting)]

use proc_macros::{external, inline_macros, with_span};

#[rustfmt::skip]
#[inline_macros]
fn main() {
    {
        let x = 42;
        let y = &42;

        let _ = x -- 30; //~ suspicious_unary_op_formatting
        let _ = x ** y; //~ suspicious_unary_op_formatting
        let _ = x +! 30; //~ suspicious_unary_op_formatting
        let _ = x <<* y; //~ suspicious_unary_op_formatting
        let _ = x ==! 30; //~ suspicious_unary_op_formatting

        let _ = x+-30;
        let _ = x +-30;
        let _ = x +-/* comment */30;
    }

    with_span! {
        span
        let x = 42;
        let _ = x -- 30;
    }

    external! {
        let x = 42;
        let _ = x -- 30;
    }

    inline! {
        let x = 42;
        let _ = x &- 30; //~ suspicious_unary_op_formatting
        let _ = x |- $30; //~ suspicious_unary_op_formatting
    }

    inline! {
        let _ = $x >- $30; //~ suspicious_unary_op_formatting
    }

    {
        macro_rules! m {
            ($($t:tt)*) => {
                let mut x = 42;
                let _ = x $($t)* 35;
            }
        }
        m!(--);
    }

    {
        macro_rules! m {
            ($($t:tt)*) => {
                let mut x = 42;
                let _ = x $($t)*;
            }
        }
        m!(-- 35);
    }
}
