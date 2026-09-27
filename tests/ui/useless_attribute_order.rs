//@no-rustfix: the suggestion rewrites `#[allow(..)]` to `#![allow(..)]`, which is only valid
// when the attribute is the first on the item. Here a permitted attribute precedes it, so the
// suggestion would emit non-compiling code. This needs a separate file because the directive
// above applies to a whole file, and `useless_attribute.rs` is a rustfix test with a `.fixed`
// file.
#![warn(clippy::useless_attribute)]

// A permitted lint level attribute must not keep the attributes after it from being checked. The
// lint used to stop checking the whole item, so the outcome depended on the order in which the
// attributes were written.
//
// This is a false negative with no tracking issue yet. It was found in the same function while
// fixing https://github.com/rust-lang/rust-clippy/issues/17783.
pub mod order_dependence {
    // `unused_imports` is permitted on `use` items, so only the following attribute is reported.
    #[allow(unused_imports)]
    #[allow(clippy::almost_swapped)]
    //~^ useless_attribute
    use std::cmp::Ordering as OrderingA;

    // The same attribute, but written before the permitted one, is reported all the same.
    #[allow(clippy::almost_swapped)]
    //~^ useless_attribute
    #[allow(unused_imports)]
    use std::cmp::Ordering as OrderingB;

    // `unused_extern_crates` is permitted on `extern crate` items.
    #[allow(unused_extern_crates)]
    #[allow(clippy::almost_swapped)]
    //~^ useless_attribute
    extern crate regex as regex_a;

    // The same attribute, but written before the permitted one, is reported all the same.
    #[allow(clippy::almost_swapped)]
    //~^ useless_attribute
    #[allow(unused_extern_crates)]
    extern crate regex as regex_b;
}

fn main() {}
