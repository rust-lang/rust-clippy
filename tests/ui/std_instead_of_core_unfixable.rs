#![warn(clippy::std_instead_of_alloc, clippy::std_instead_of_core)]

#[rustfmt::skip]
fn issue14982() {
    use std::{collections::HashMap, hash::Hash};
    //~^ std_instead_of_core
}

#[rustfmt::skip]
fn issue15143() {
    use std::{error::Error, vec::Vec, fs::File};
    //~^ std_instead_of_core
    //~| std_instead_of_alloc
}

#[rustfmt::skip]
fn pr16964() {
    use std::{
        borrow::Cow,
        //~^ std_instead_of_alloc
        collections::BTreeSet,
        //~^ std_instead_of_alloc
        ffi::OsString,
    };
}

#[warn(clippy::alloc_instead_of_core)]
fn issue15579() {
    use std::alloc;

    let layout = alloc::Layout::new::<u8>();
    //~^ std_instead_of_core
}
