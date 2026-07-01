//@no-rustfix
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

#[warn(clippy::alloc_instead_of_core)]
#[rustfmt::skip]
fn issue11159() {
    use std::fmt;
    //~^ std_instead_of_alloc

    struct S;

    impl fmt::Display for S {
        //~^ alloc_instead_of_core
        fn fmt(
            &self,
            _: &mut fmt::Formatter<'_>
            //~^ alloc_instead_of_core
        ) -> fmt::Result {
            //~^ alloc_instead_of_core
            todo!()
        }
    }
}

fn issue15579() {
    use std::alloc;

    let layout = alloc::Layout::new::<u8>();
    //~^ std_instead_of_core
}
