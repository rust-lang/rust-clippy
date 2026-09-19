//@aux-build:wildcard_imports_helper.rs

#![feature(test)]
#![warn(clippy::std_wildcard_imports)]

extern crate alloc;
extern crate proc_macro;
extern crate std as standard;
extern crate test as test_crate;
extern crate wildcard_imports_helper;

use alloc::boxed::*;
//~^ std_wildcard_imports
use core::cell::*;
//~^ std_wildcard_imports
use proc_macro::*;
//~^ std_wildcard_imports
use std::any::*;
//~^ std_wildcard_imports
use standard::mem::{swap, *};
//~^ std_wildcard_imports
use test_crate::bench::*;
//~^ std_wildcard_imports
use ::std::fs::*;
//~^ std_wildcard_imports
use wildcard_imports_helper::fmt::*;
//~^ std_wildcard_imports

use std::io::prelude::*;

pub mod reexports {
    pub use std::fmt::*;

    pub fn use_debug(_: &dyn Debug) {}
}

use wildcard_imports_helper::*;

macro_rules! import_fmt {
    () => {
        use std::fmt::*;
    };
}

mod macro_expansion {
    import_fmt!();

    pub fn use_display(_: &dyn Display) {}
}

mod local_std {
    mod std {
        pub mod things {
            pub fn item() {}
        }
    }

    use std::things::*;

    pub fn use_item() {
        item();
    }
}

// This is intentionally checked by `enum_glob_use`, not this lint.
use std::cmp::Ordering::*;

mod both_lints {
    #![warn(clippy::wildcard_imports)]

    use std::rc::*;
    //~^ std_wildcard_imports

    pub fn new_rc() -> Rc<()> {
        Rc::new(())
    }
}

struct Reader;

impl Read for Reader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

fn main() {
    let _ = Box::new(());
    let _ = Cell::new(());
    let _ = is_available();
    let _ = type_name::<i32>();

    let mut a = 1;
    let mut b = 2;
    swap(&mut a, &mut b);
    let _ = align_of::<i32>();

    black_box(());
    let _ = File::open("does-not-exist");
    let _: &dyn Debug = &0;
    let _ = Reader;
    extern_foo();
    macro_expansion::use_display(&0);
    local_std::use_item();
    let _ = Less;
    let _ = both_lints::new_rc();
}
