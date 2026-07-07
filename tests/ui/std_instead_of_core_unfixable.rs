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

#[rustfmt::skip]
fn issue12468() {
    use std::{
        fmt::Result,
        //~^ std_instead_of_core
        io::Write,
    };

    use std::sync::{
        Arc,
        //~^ std_instead_of_alloc
        Mutex,
    };
}

#[allow(clippy::legacy_numeric_constants)]
#[warn(clippy::alloc_instead_of_core)]
#[rustfmt::skip]
fn pr17252_large() {
    extern crate alloc;

    use {
        std::sync::Mutex,
        ::{
            std::{
                fmt::{*, Formatter},
                //~^ std_instead_of_alloc
                //~| std_instead_of_core
                fs,
                //~v std_instead_of_core
                sync::atomic,
                //~v std_instead_of_core
                sync::atomic::{
                    AtomicUsize,
                    AtomicU8,
                    Ordering,
                },
            },
            core::u32,
            std::collections::HashMap,
        },
        alloc::{
            fmt::Result as FmtResult,
            //~^ alloc_instead_of_core
            format,
        }
    };
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
            _: &mut fmt::Formatter<'_>,
            //~^ alloc_instead_of_core
        ) -> fmt::Result {
            //~^ alloc_instead_of_core
            todo!()
        }
    }
}

#[warn(clippy::alloc_instead_of_core)]
#[rustfmt::skip]
fn mixed_name_spaces() {
    extern crate alloc;

    use std::{
        io,
        iter::repeat_with,
        //~^ std_instead_of_core
    };

    use alloc::alloc::{
        alloc,
        dealloc,
        Layout,
        //~^ alloc_instead_of_core
    };
}
