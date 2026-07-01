#![warn(clippy::missing_trait_methods)]

struct S {}

impl Drop for S {
    fn drop(&mut self) {}
}

//@check-pass
