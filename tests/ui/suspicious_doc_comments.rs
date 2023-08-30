#![allow(unused)]
#![warn(clippy::suspicious_doc_comments)]

//! Real module documentation.
///! Fake module documentation.
//~^ ERROR: this is an outer doc comment and does not apply to the parent module or crate
//~| NOTE: `-D clippy::suspicious-doc-comments` implied by `-D warnings`
fn baz() {}

pub mod singleline_outer_doc {
    ///! This module contains useful functions.
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr

    pub fn bar() {}
}

pub mod singleline_inner_doc {
    //! This module contains useful functions.

    pub fn bar() {}
}

pub mod multiline_outer_doc {
    /**! This module contains useful functions.
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr
     */

    pub fn bar() {}
}

pub mod multiline_inner_doc {
    /*! This module contains useful functions.
     */

    pub fn bar() {}
}

pub mod multiline_outer_doc2 {
    ///! This module
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr
    ///! contains
    ///! useful functions.

    pub fn bar() {}
}

pub mod multiline_outer_doc3 {
    ///! a
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr
    ///! b

    /// c
    pub fn bar() {}
}

pub mod multiline_outer_doc4 {
    ///! a
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr
    /// b
    pub fn bar() {}
}

pub mod multiline_outer_doc_gap {
    ///! a
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr

    ///! b
    pub fn bar() {}
}

pub mod multiline_outer_doc_commented {
    /////! This outer doc comment was commented out.
    pub fn bar() {}
}

pub mod outer_doc_macro {
    ///! Very cool macro
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr
    macro_rules! x {
        () => {};
    }
}

pub mod useless_outer_doc {
    ///! Huh.
    //~^ ERROR: this is an outer doc comment and does not apply to the parent module or cr
    use std::mem;
}

fn main() {}
