//@aux-build:../auxiliary/proc_macros.rs
#![deny(clippy::suspicious_else_formatting)]
#![allow(
    clippy::if_same_then_else,
    clippy::let_unit_value,
    clippy::needless_ifs,
    clippy::needless_else
)]

use proc_macros::{external, with_span};

fn foo() -> bool {
    true
}

#[rustfmt::skip]
fn main() {
    //~vv suspicious_else_formatting
    if foo() {
    } else
    {
    }

    // This is fine, though weird. Allman style braces on the else.
    if foo() {
    }
    else
    {
    }

    //~vv suspicious_else_formatting
    if foo() {
    } else
    if foo() {
    }

    //~vv suspicious_else_formatting
    if foo() {
    }
    else
    if foo() {
    }

    // those are ok:
    if foo() {
    } else {
    }

    if foo() {
    }
    else {
    }

    //~vv suspicious_else_formatting
    if foo() {
    }

    else
    {

    }

    //~vv suspicious_else_formatting
    if foo() {
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

    // #12497 Don't trigger lint as rustfmt wants it
    if true {
        println!("true");
    }
    /*else if false {
}*/
    else {
        println!("false");
    }

    if true {
        println!("true");
    } // else if false {}
    else {
        println!("false");
    }

    if true {
        println!("true");
    } /* if true {
        println!("true");
}
    */
    else {
        println!("false");
    }

    with_span! {
        span
        if true {
            let _ = 0;
        } else

        {
            let _ = 1;
        }
    }


    external! {
        if true {
            let _ = 0;
        } else

        {
            let _ = 1;
        }
    }

    //~vvv suspicious_else_formatting
    if true {
        let _ = 0;
    } /* comment */ else

    {
        let _ = 1;
    }

    //~vvv suspicious_else_formatting
    if true {
        let _ = 0;
    }
    // comment
    else


    {
        let _ = 1;
    }


    //~vvv suspicious_else_formatting
    if true {
        let _ = 0;
    } /*
       * some comment */ else
    {
        let _ = 1;
    }
}
