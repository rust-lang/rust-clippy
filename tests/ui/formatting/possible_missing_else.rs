//@aux-build:../auxiliary/proc_macros.rs
#![deny(clippy::possible_missing_else)]
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
    //~vv possible_missing_else
    if foo() {
    } {
    }

    //~vv possible_missing_else
    if foo() {
    } if foo() {
    }

    let _ = {
        let _ = 0;

        //~vv possible_missing_else
        if foo() {
        } if foo() {
        }
        else {
        }
    };

    let _ = {
        //~vv possible_missing_else
        if foo() {
        } if foo() {
        }
        else {
        }

        let _ = 0;
    };

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
}
