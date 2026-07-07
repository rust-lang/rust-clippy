#![warn(clippy::std_instead_of_alloc)]
#![allow(unused_imports)]

const _: () = {
    extern crate alloc;
};

#[rustfmt::skip]
fn issue16695() {
    use std::collections::VecDeque;
    //~^ std_instead_of_alloc
}
