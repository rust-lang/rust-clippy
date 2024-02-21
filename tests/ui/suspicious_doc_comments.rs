#![allow(unused)]
#![warn(clippy::suspicious_doc_comments)]

//! Real module documentation.
//~v suspicious_doc_comments
///! Fake module documentation.
fn baz() {}

pub mod singleline_outer_doc {
    //~v suspicious_doc_comments
    ///! This module contains useful functions.

    pub fn bar() {}
}

pub mod singleline_inner_doc {
    //! This module contains useful functions.

    pub fn bar() {}
}

pub mod multiline_outer_doc {
    //~v suspicious_doc_comments
    /**! This module contains useful functions.
     */

    pub fn bar() {}
}

pub mod multiline_inner_doc {
    /*! This module contains useful functions.
     */

    pub fn bar() {}
}

pub mod multiline_outer_doc2 {
    //~v suspicious_doc_comments
    ///! This module
    ///! contains
    ///! useful functions.

    pub fn bar() {}
}

pub mod multiline_outer_doc3 {
    //~v suspicious_doc_comments
    ///! a
    ///! b

    /// c
    pub fn bar() {}
}

pub mod multiline_outer_doc4 {
    //~v suspicious_doc_comments
    ///! a
    /// b
    pub fn bar() {}
}

pub mod multiline_outer_doc_gap {
    //~v suspicious_doc_comments
    ///! a

    ///! b
    pub fn bar() {}
}

pub mod multiline_outer_doc_commented {
    /////! This outer doc comment was commented out.
    pub fn bar() {}
}

pub mod outer_doc_macro {
    //~v suspicious_doc_comments
    ///! Very cool macro
    macro_rules! x {
        () => {};
    }
}

pub mod useless_outer_doc {
    //~v suspicious_doc_comments
    ///! Huh.
    use std::mem;
}

fn main() {}
