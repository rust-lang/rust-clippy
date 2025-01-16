#![warn(clippy::raw_assign_to_drop)]
#![allow(clippy::missing_safety_doc)]

use std::cell::UnsafeCell;

pub unsafe fn foo(r: *mut String, i: *mut i32) {
    unsafe {
        *r = "foo".to_owned();

        // no lint on {integer}
        *i = 47;

        (*r, *r) = ("foo".to_owned(), "bar".to_owned());

        (*r, *i) = ("foo".to_owned(), 47);

        let mut x: String = Default::default();
        *(&mut x as *mut _) = "Foo".to_owned();

        // no lint on `u8`
        *x.as_mut_ptr() = b'a';

        let mut v: Vec<String> = vec![];
        *v.as_mut_ptr() = Default::default();
    }
}

pub unsafe fn unsafecell() {
    // No lint
    let c = UnsafeCell::new(String::new());
    unsafe {
        *c.get() = String::new();
    }
}

fn main() {}
