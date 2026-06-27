#![warn(clippy::missing_trait_methods)]
#![allow(incomplete_features)]
#![cfg_attr(feature = "foo", feature(pin_ergonomics))]

#[cfg(feature = "foo")]
mod foo {
    struct S {}

    impl Drop for S {
        // FIXME: this should be diagnosed
        fn drop(&mut self) {}
    }
}

mod bar {
    struct S {}

    impl Drop for S {
        // no-diagnostic
        fn drop(&mut self) {}
    }
}

//@check-pass
