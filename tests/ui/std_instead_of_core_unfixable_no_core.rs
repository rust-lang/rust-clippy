#![warn(clippy::std_instead_of_core)]
#![allow(unused_imports)]
#![no_implicit_prelude]

const _: () = {
    extern crate core;
};

extern crate std;

fn issue15836() {
    use std::cmp::{self, Eq, Ordering, PartialEq, PartialOrd};
    //~^ std_instead_of_core
    use std::option::Option::{self, Some};
    //~^ std_instead_of_core
    use std::todo;
    //~^ std_instead_of_core
}
