#![warn(clippy::missing_trait_methods)]
#![allow(incomplete_features)]
#![feature(pin_ergonomics)]

struct S {}

impl Drop for S {
    // FIXME: this should be diagnosed
    fn drop(&mut self) {}
}

//@check-pass
