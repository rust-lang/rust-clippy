#![warn(clippy::rc_buffer)]
#![allow(dead_code, unused_imports)]

use std::cell::RefCell;
use std::ffi::OsString;
use std::path::PathBuf;
use std::rc::Rc;

struct S {
    // triggers lint
    bad1: Rc<String>,
    //~^ ERROR: usage of `Rc<T>` when T is a buffer type
    //~| NOTE: `-D clippy::rc-buffer` implied by `-D warnings`
    bad2: Rc<PathBuf>,
    //~^ ERROR: usage of `Rc<T>` when T is a buffer type
    bad3: Rc<Vec<u8>>,
    //~^ ERROR: usage of `Rc<T>` when T is a buffer type
    bad4: Rc<OsString>,
    //~^ ERROR: usage of `Rc<T>` when T is a buffer type
    // does not trigger lint
    good1: Rc<RefCell<String>>,
}

// triggers lint
fn func_bad1(_: Rc<String>) {}
//~^ ERROR: usage of `Rc<T>` when T is a buffer type
fn func_bad2(_: Rc<PathBuf>) {}
//~^ ERROR: usage of `Rc<T>` when T is a buffer type
fn func_bad3(_: Rc<Vec<u8>>) {}
//~^ ERROR: usage of `Rc<T>` when T is a buffer type
fn func_bad4(_: Rc<OsString>) {}
//~^ ERROR: usage of `Rc<T>` when T is a buffer type
// does not trigger lint
fn func_good1(_: Rc<RefCell<String>>) {}

fn main() {}
