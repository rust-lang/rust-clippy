#![feature(rustc_private)]
#![warn(clippy::needless_impl_lint_pass)]

extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;

use rustc_session::{declare_lint_pass, impl_lint_pass};

mod semi {
    use super::*;

    struct WithoutFields;

    impl_lint_pass!(WithoutFields => []);
    //~^ needless_impl_lint_pass
}

mod braces {
    use super::*;

    struct WithoutFields {}

    impl_lint_pass!(WithoutFields => []);
    //~^ needless_impl_lint_pass
}

mod after_macro {
    use super::*;

    impl_lint_pass!(WithoutFields => []);
    //~^ needless_impl_lint_pass

    struct WithoutFields;
}

mod has_fields {
    use super::*;
    use clippy_utils::msrvs::Msrv;

    struct WithFields {
        msrv: Msrv,
    }

    // don't lint: can't use `declare_lint_pass!` because of the field
    impl_lint_pass!(WithFields => []);
}

// don't lint: `impl_lint_pass!` not written by the user
declare_lint_pass!(NoFields2 => []);
