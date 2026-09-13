//@aux-build:proc_macros.rs
#![warn(clippy::invalid_nonzero)]

extern crate proc_macros;

use std::num::{NonZero, NonZeroI32, NonZeroU8, NonZeroU16, NonZeroUsize};

const ZERO: u8 = 0;

fn main() {
    let _ = NonZeroU16::new(0);
    //~^ invalid_nonzero
    let _ = NonZeroU8::new(ZERO);
    //~^ invalid_nonzero
    let _ = NonZeroI32::new(0);
    //~^ invalid_nonzero
    let _ = NonZero::<usize>::new(0);
    //~^ invalid_nonzero
    let _: Option<NonZeroUsize> = NonZero::new(0);
    //~^ invalid_nonzero

    // Should not lint.
    let _ = NonZeroU16::new(1);
    let _ = NonZeroU16::new(1).unwrap();
    let x = 5;
    let _ = NonZeroU8::new(x);

    proc_macros::external! {
        let _ = NonZeroU16::new(0);
    }
    proc_macros::with_span! {
        span
        let _ = NonZeroU16::new(0);
    }
}
