#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![allow(unused_imports)]

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
#[rustfmt::skip]
fn pr17252() {
    extern crate alloc;

    use alloc::str::{self as _, FromStr as _};
    //~^ alloc_instead_of_core

    use std::sync::{
        Arc,
        Mutex,
        Weak,
        atomic::{
            AtomicPtr,
            Ordering,
        },
    };
    //~^^^^^^^^ std_instead_of_alloc
    //~^^^^^^^ std_instead_of_alloc
    //~^^^^^^ std_instead_of_core
    //~^^^^^^ std_instead_of_core
}
