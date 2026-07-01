#![warn(clippy::missing_trait_methods)]
#![allow(incomplete_features)]
#![cfg_attr(use_pin, feature(pin_ergonomics))]

//@compile-flags: --cfg=use_pin

#[cfg(use_pin)]
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
