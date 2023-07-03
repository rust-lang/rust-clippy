//@aux-build:proc_macros.rs:proc-macro
#![allow(clippy::no_effect, unused)]
#![warn(clippy::bare_dos_device_names)]

#[macro_use]
extern crate proc_macros;

use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

fn a<T: AsRef<Path>>(t: T) {}

fn b(t: impl AsRef<Path>) {}

// TODO: More tests for traits.

fn main() {
    a("con");
    b("conin$");
    File::open("conin$");
    std::path::PathBuf::from("a");
    PathBuf::from("a");
    Path::new("a");
    external! {
        a("con");
    }
    with_span! {
        span
        a("con");
    }
}
