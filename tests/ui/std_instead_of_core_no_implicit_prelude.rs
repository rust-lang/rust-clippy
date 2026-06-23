#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![allow(unused_imports)]
#![no_implicit_prelude]

extern crate alloc;
extern crate core;
extern crate std;

fn pr17252() {
    use std::result::Result;
    //~^ std_instead_of_core

    use std::vec::Vec;
    //~^ std_instead_of_alloc
}
