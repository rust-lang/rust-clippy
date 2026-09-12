//@aux-build:proc_macros.rs
//@no-rustfix
#![deny(clippy::deprecated_attributes_without_since)]

extern crate proc_macros;
use proc_macros::{external, with_span};

fn main() {
    // These should trigger the lint
    #[deprecated]
    //~^ deprecated_attributes_without_since
    fn no_fields() {}

    #[deprecated(note = "I don't feel like maintaining this anymore, sorry")]
    //~^ deprecated_attributes_without_since
    fn has_note() {}

    #[deprecated = "probably a bad idea to use this"]
    //~^ deprecated_attributes_without_since
    fn has_note_eq() {}

    // These should be fine
    #[allow(deprecated)]
    #[allow(dead_code, reason = "This should be allowed")]
    #[expect(dead_code)]
    #[warn(dyn_drop, reason = "Warnings can also have reasons")]
    #[warn(redundant_lifetimes)]
    #[deny(deref_nullptr)]
    #[forbid(deref_nullptr)]
    fn correct_attribute_only() {}

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
    fn has_both_since_first() {}

    #[deprecated(note = "use has_since instead", since = "0.0.1")]
    fn has_both_note_first() {}

    #[deprecated(since = "1.42.100")]
    fn has_since() {}
}
