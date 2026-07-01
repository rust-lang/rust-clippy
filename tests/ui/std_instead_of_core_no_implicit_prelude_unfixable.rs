#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![allow(unused_imports)]
#![no_implicit_prelude]

extern crate std;

fn pr17252_no_core() {
    // This wont have a suggestion since `core` isn't in scope
    use std::result::Result;
    //~^ std_instead_of_core
}

fn pr17252_no_alloc() {
    // This wont have a suggestion since `alloc` isn't in scope
    use std::vec::Vec;
    //~^ std_instead_of_alloc
}

fn pr17252_local_core() {
    extern crate core;

    // FIXME: This *should* have a suggestion, but we're not currently
    // checking local scope, only the extern crate prelude.
    use std::result::Result;
    //~^ std_instead_of_core
}
