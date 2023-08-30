#![warn(clippy::rc_buffer)]
#![allow(dead_code, unused_imports)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct S {
    // triggers lint
    bad1: Arc<String>,
    //~^ ERROR: usage of `Arc<T>` when T is a buffer type
    //~| NOTE: `-D clippy::rc-buffer` implied by `-D warnings`
    bad2: Arc<PathBuf>,
    //~^ ERROR: usage of `Arc<T>` when T is a buffer type
    bad3: Arc<Vec<u8>>,
    //~^ ERROR: usage of `Arc<T>` when T is a buffer type
    bad4: Arc<OsString>,
    //~^ ERROR: usage of `Arc<T>` when T is a buffer type
    // does not trigger lint
    good1: Arc<Mutex<String>>,
}

// triggers lint
fn func_bad1(_: Arc<String>) {}
//~^ ERROR: usage of `Arc<T>` when T is a buffer type
fn func_bad2(_: Arc<PathBuf>) {}
//~^ ERROR: usage of `Arc<T>` when T is a buffer type
fn func_bad3(_: Arc<Vec<u8>>) {}
//~^ ERROR: usage of `Arc<T>` when T is a buffer type
fn func_bad4(_: Arc<OsString>) {}
//~^ ERROR: usage of `Arc<T>` when T is a buffer type
// does not trigger lint
fn func_good1(_: Arc<Mutex<String>>) {}

fn main() {}
