// run-rustfix
// aux-build:proc_macros.rs

#![allow(clippy::return_self_not_must_use)]
#![warn(clippy::deref_addrof)]

extern crate proc_macros;
use proc_macros::inline_macros;

fn get_number() -> usize {
    10
}

fn get_reference(n: &usize) -> &usize {
    n
}

#[allow(clippy::double_parens)]
#[allow(unused_variables, unused_parens)]
fn main() {
    let a = 10;
    let aref = &a;

    let b = *&a;

    let b = *&get_number();

    let b = *get_reference(&a);

    let bytes: Vec<usize> = vec![1, 2, 3, 4];
    let b = *&bytes[1..2][0];

    //This produces a suggestion of 'let b = (a);' which
    //will trigger the 'unused_parens' lint
    let b = *&(a);

    let b = *(&a);

    #[rustfmt::skip]
    let b = *((&a));

    let b = *&&a;

    let b = **&aref;

    let _ = unsafe { *core::ptr::addr_of!(a) };
}

#[derive(Copy, Clone)]
pub struct S;
#[inline_macros]
impl S {
    pub fn f(&self) -> &Self {
        inline!(*& $(@expr self))
    }
    #[allow(unused_mut)] // mut will be unused, once the macro is fixed
    pub fn f_mut(mut self) -> Self {
        inline!(*&mut $(@expr self))
    }
}
