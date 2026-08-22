//@aux-build:proc_macros.rs
//@no-rustfix
#![deny(clippy::deprecated_attributes_without_note)]

extern crate proc_macros;
use proc_macros::{external, with_span};

fn main() {
    // These should trigger the lint
    #[deprecated]
    //~^ deprecated_attributes_without_note
    fn foo() {}

    #[deprecated(since = "1.42.100")]
    //~^ deprecated_attributes_without_note
    fn quux() {}

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

    #[deprecated(since = "TBD", note = "use qux instead")]
    fn baz() {}

    #[deprecated(note = "use quux instead", since = "0.0.1")]
    fn qux() {}

    #[deprecated(note = "I don't feel like maintaining this anymore, sorry")]
    fn bar() {}

    #[deprecated = "probably a bad idea to use this"]
    fn weird() {}
}
