#![warn(clippy::doc_attr_ordering)]

extern crate proc_macro;

use proc_macro::TokenStream;

#[repr(u8)]
/// Type
//~^ doc_attr_ordering
enum DocAttrOrdering {
    Variant,
}

#[must_use]
/// Function
//~^ doc_attr_ordering
fn doc_attr_ordering() -> bool {
    #[allow(unused_doc_comments)]
    /// Statement
    //~^ doc_attr_ordering
    true
}

#[proc_macro]
/// Proc macro
//~^ doc_attr_ordering
pub fn proc_macro_fn(t: TokenStream) -> TokenStream {
    t
}

/// Documentation that...
#[must_use]
/// ...wraps other attributes
//~^ doc_attr_ordering
struct OtherInterspersed;

#[must_use]
/// Documentation in between other attributes
//~^ doc_attr_ordering
#[repr(u8)]
enum DocInterspersed {
    Variant,
}

/// Documentation
#[doc = concat!("can include", " doc attributes")]
/// and continue afterwards safely
#[repr(u8)]
enum DocAttrInterspersed {
    Variant,
}

/// Using `cfg_attr`  
#[cfg_attr(true, doc = "interspersed with doc comments")]
/// counts as a doc comment
struct CfgAttrDoc;

mod nested {
    #![allow(dead_code)]
    //! Module/outer attributes are not checked.
}

#[allow(dead_code)]
/// Only outer attributes are checked
//~^ doc_attr_ordering
mod inner_outer {
    #![allow(clippy::mixed_attributes_style)]
    //! ...even when the inner is present.
}

#[derive(Default)]
/// A `derive` macro is erased during macro expansion,
/// so this will not be flagged.
struct DerivedException;

#[cfg_attr(false, derive(Default))]
/// A disabled `cfg_attr` is not caught because it is erased.
struct DisabledCfgAttr;
