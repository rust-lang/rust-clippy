//@aux-build:proc_macro_suspicious_else_formatting.rs

#![warn(clippy::suspicious_else_formatting)]
#![allow(
    clippy::if_same_then_else,
    clippy::let_unit_value,
    clippy::needless_if,
    clippy::needless_else
)]

extern crate proc_macro_suspicious_else_formatting;
use proc_macro_suspicious_else_formatting::DeriveBadSpan;

fn foo() -> bool {
    true
}

#[rustfmt::skip]
fn main() {
    // weird `else` formatting:
    if foo() {
    //~v suspicious_else_formatting
    } {
    }

    if foo() {
    //~v suspicious_else_formatting
    } if foo() {
    }

    let _ = { // if as the last expression
        let _ = 0;

        if foo() {
        //~v suspicious_else_formatting
        } if foo() {
        }
        else {
        }
    };

    let _ = { // if in the middle of a block
        if foo() {
        //~v suspicious_else_formatting
        } if foo() {
        }
        else {
        }

        let _ = 0;
    };

    if foo() {
    //~v suspicious_else_formatting
    } else
    {
    }

    // This is fine, though weird. Allman style braces on the else.
    if foo() {
    }
    else
    {
    }

    if foo() {
    //~v suspicious_else_formatting
    } else
    if foo() { // the span of the above error should continue here
    }

    if foo() {
    //~v suspicious_else_formatting
    }
    else
    if foo() { // the span of the above error should continue here
    }

    // those are ok:
    if foo() {
    }
    {
    }

    if foo() {
    } else {
    }

    if foo() {
    }
    else {
    }

    if foo() {
    }
    if foo() {
    }

    // Almost Allman style braces. Lint these.
    if foo() {
    //~v suspicious_else_formatting
    }

    else
    {

    }

    if foo() {
    //~v suspicious_else_formatting
    }
    else

    {

    }

    // #3864 - Allman style braces
    if foo()
    {
    }
    else
    {
    }

    //#10273 This is fine. Don't warn
    if foo() {
    } else
    /* whelp */
    {
    }
}

// #7650 - Don't lint. Proc-macro using bad spans for `if` expressions.
#[derive(DeriveBadSpan)]
struct _Foo(u32, u32);
