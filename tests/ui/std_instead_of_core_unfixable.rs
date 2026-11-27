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

#[rustfmt::skip]
fn issue18081() {
    use std::{future::Future, sync::Mutex};
    //~^ std_instead_of_core
    use std::{future::Future as _, io::ErrorKind};
    //~^ std_instead_of_core
    use std::{future::Future as _, println};
    //~^ std_instead_of_core
    use std::{future::Future as _, format};
    //~^ std_instead_of_core
    use std::{format as format2, future::Future as _};
    //~^ std_instead_of_core
    use std::{future::Future as _, fs::*};
    //~^ std_instead_of_core
    use std::{future::Future as _, fs::{}};
    //~^ std_instead_of_core
}

#[clippy::msrv = "1.76"]
#[rustfmt::skip]
fn issue18081_msrv() {
    use std::{fmt::Debug, net::IpAddr};
    //~^ std_instead_of_core
}
