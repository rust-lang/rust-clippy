//@aux-build:proc_macros.rs
//@no-rustfix
#![deny(clippy::deprecated_attributes_without_since)]
#![allow(dead_code, deprecated)]

extern crate proc_macros;
use proc_macros::{external, with_span};

// === Public items: should trigger the lint ===

#[deprecated]
//~^ deprecated_attributes_without_since
pub fn no_fields() {}

#[deprecated(note = "I don't feel like maintaining this anymore, sorry")]
//~^ deprecated_attributes_without_since
pub fn has_note() {}

#[deprecated = "probably a bad idea to use this"]
//~^ deprecated_attributes_without_since
pub fn has_note_eq() {}

#[deprecated = ""]
//~^ deprecated_attributes_without_since
pub fn empty_note() {}

#[cfg_attr(true, deprecated)]
//~^ deprecated_attributes_without_since
pub fn cfg_true_no_fields() {}

#[cfg_attr(true, deprecated(note = "look down!"))]
//~^ deprecated_attributes_without_since
pub fn cfg_true_has_note() {}

#[cfg_attr(true, deprecated = "the note below me always lies")]
//~^ deprecated_attributes_without_since
pub fn cfg_true_has_note_eq() {}

#[deprecated(note = "kinda weird to deprecate everything inside too")]
//~^ deprecated_attributes_without_since
pub mod module {
    #[deprecated]
    //~^ deprecated_attributes_without_since
    pub struct DeprecatedStruct {
        #[deprecated]
        //~^ deprecated_attributes_without_since
        pub deprecated_field: u32,
    }

    #[deprecated]
    impl DeprecatedStruct {
        #[deprecated]
        //~^ deprecated_attributes_without_since
        pub fn deprecated_method() {}
    }

    #[deprecated]
    //~^ deprecated_attributes_without_since
    pub trait DeprecatedTrait {
        #[deprecated]
        //~^ deprecated_attributes_without_since
        fn trait_method();

        #[deprecated]
        //~^ deprecated_attributes_without_since
        fn default_trait_method() {}
    }
}

// === Non-public items: should NOT trigger the lint ===

#[deprecated]
fn private_no_fields() {}

#[deprecated(note = "internal use")]
fn private_has_note() {}

#[deprecated]
pub(crate) fn pub_crate_fn() {}

mod private_mod {
    #[deprecated]
    pub(super) fn pub_super_fn() {}

    #[deprecated]
    pub(crate) fn restricted_fn() {}

    #[deprecated]
    fn private_fn() {}

    #[deprecated]
    struct PrivateStruct;

    impl PrivateStruct {
        #[deprecated]
        fn private_method() {}
    }
}

// === Items that are correctly annotated: no lint ===

external! {
    #[deprecated]
    fn a() {}
}
with_span! {
    span
    #[deprecated]
    fn b() {}
}

#[deprecated(since = "TBD", note = "use has_both_note_first instead")]
pub fn has_both_since_first() {}

#[deprecated(note = "use has_since instead", since = "0.0.1")]
pub fn has_both_note_first() {}

#[deprecated(since = "1.42.100")]
pub fn has_since() {}

#[cfg_attr(true, deprecated(since = "0.0.0"))]
pub fn cfg_true_has_since() {}

#[cfg_attr(false, deprecated(since = "255.255.255"))]
fn cfg_false_has_since() {}

#[cfg_attr(false, deprecated)]
fn cfg_false_no_fields() {}

#[cfg_attr(true, deprecated(note = "use cfg_false_has_both instead", since = "16.16.16"))]
pub fn cfg_true_has_both() {}

#[cfg_attr(false, deprecated(note = "use cfg_true_has_both instead", since = "19.19.19"))]
fn cfg_false_has_both() {}

#[cfg_attr(false, deprecated(note = "look up!"))]
fn cfg_false_has_note() {}

#[cfg_attr(false, deprecated = "the note above me never lies")]
fn cfg_false_has_note_eq() {}

fn main() {}
