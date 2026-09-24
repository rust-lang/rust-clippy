//@revisions: edition2015 edition2021
//@[edition2015] edition:2015
//@[edition2021] edition:2021
#![warn(clippy::missing_panic_message)]

use std::hint::black_box;
use std::panic::panic_any;

fn main() {
    panic!(); //~ missing_panic_message
    todo!(); //~ missing_panic_message
    unimplemented!(); //~ missing_panic_message
    unreachable!(); //~ missing_panic_message

    panic!("r");
    todo!("u");
    unimplemented!("s");
    unreachable!("t");

    let _ = vec![0];

    macro_rules! ignore_me {
        () => {{
            panic!();
            todo!();
            unimplemented!();
            unreachable!();
        }};
    }
    ignore_me!();
}

fn qualified_macros() {
    std::panic!(); //~ missing_panic_message
    core::panic!(); //~ missing_panic_message

    std::todo!(); //~ missing_panic_message
    core::prelude::v1::todo!(); //~ missing_panic_message

    unimplemented!(); //~ missing_panic_message
    core::prelude::v1::unimplemented!(); //~ missing_panic_message

    unreachable!(); //~ missing_panic_message
    core::prelude::v1::unreachable!(); //~ missing_panic_message
}
