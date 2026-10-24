//@check-pass
#![expect(clippy::missing_const_for_thread_local)]

use std::cell::Cell;

macro_rules! constant_initializer {
    () => {
        const { Cell::new(0) }
    };
}

thread_local! {
    static VALUE: Cell<u8> = constant_initializer!();
}

fn main() {}
