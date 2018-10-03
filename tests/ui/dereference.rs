#![feature(tool_lints)]

use std::ops::{Deref, DerefMut};

#[allow(clippy::many_single_char_names, clippy::double_parens)]
#[allow(unused_variables)]
#[warn(clippy::deref_method_explicit)]
fn main() {
    let mut a: String = String::from("foo");

    {
        let aref = &a;
        let b = aref.deref();
    }

    {
        let aref = &mut a;
        let b = aref.deref_mut();
    }
}
