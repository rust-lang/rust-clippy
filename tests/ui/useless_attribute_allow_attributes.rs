//@no-rustfix: the suggestion rewrites `#[warn(..)]` to `#![warn(..)]`, which is only a crate or
// block level attribute. Turning these into inner attributes would not compile, so they need a
// separate file. `useless_attribute.rs` cannot host them, as it is a rustfix test with a `.fixed`
// file, and this directive applies to a whole file.
#![warn(clippy::useless_attribute)]
#![feature(rustc_private)]

// `clippy::allow_attributes` is emitted on the item's own `allow` attributes, not on the item
// itself. Naming it is therefore only useless when the item has no `allow` for it to fire on.
mod emittable {
    // the workaround for an `allow` that is only useless for some expansions of a macro
    #[expect(clippy::allow_attributes, reason = "lint depends on macro application")]
    #[allow(unused_imports)]
    use std::collections::HashMap;

    // it fires the same way on an `extern crate`, so the workaround is not useless there either
    #[expect(clippy::allow_attributes, reason = "lint depends on macro application")]
    #[allow(unused_extern_crates)]
    extern crate core as re;
}

mod not_emittable {
    // nothing for `clippy::allow_attributes` to fire on
    #[warn(clippy::allow_attributes)]
    //~^ useless_attribute
    use std::collections::BTreeMap;

    // `clippy::useless_attribute` is only emitted on a sibling lint level attribute, and an
    // `allow` naming it can only ever suppress itself
    #[warn(clippy::useless_attribute)]
    //~^ useless_attribute
    #[allow(unused_imports)]
    use std::collections::HashSet;
}
