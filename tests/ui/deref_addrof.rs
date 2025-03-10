//@aux-build:proc_macros.rs

#![allow(clippy::return_self_not_must_use, clippy::useless_vec)]
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
    //~^ deref_addrof

    let b = *&get_number();
    //~^ deref_addrof

    let b = *get_reference(&a);

    let bytes: Vec<usize> = vec![1, 2, 3, 4];
    let b = *&bytes[1..2][0];
    //~^ deref_addrof

    //This produces a suggestion of 'let b = (a);' which
    //will trigger the 'unused_parens' lint
    let b = *&(a);
    //~^ deref_addrof

    let b = *(&a);
    //~^ deref_addrof

    #[rustfmt::skip]
    let b = *((&a));
    //~^ deref_addrof

    let b = *&&a;
    //~^ deref_addrof

    let b = **&aref;
    //~^ deref_addrof

    let _ = unsafe { *core::ptr::addr_of!(a) };

    let _repeat = *&[0; 64];
    //~^ deref_addrof
    // do NOT lint for array as semantic differences with/out `*&`.
    let _arr = *&[0, 1, 2, 3, 4];
}

#[derive(Copy, Clone)]
pub struct S;
#[inline_macros]
impl S {
    pub fn f(&self) -> &Self {
        inline!(*& $(@expr self))
        //~^ deref_addrof
    }
    #[allow(unused_mut)] // mut will be unused, once the macro is fixed
    pub fn f_mut(mut self) -> Self {
        inline!(*&mut $(@expr self))
        //~^ deref_addrof
    }
}

fn issue14386() {
    use std::mem::ManuallyDrop;

    #[derive(Copy, Clone)]
    struct Data {
        num: u64,
    }

    union DataWithPadding {
        data: ManuallyDrop<Data>,
        prim: ManuallyDrop<u64>,
        padding: [u8; size_of::<Data>()],
        tup: (ManuallyDrop<Data>, ()),
    }

    let mut a = DataWithPadding {
        padding: [0; size_of::<DataWithPadding>()],
    };
    unsafe {
        (*(&raw mut a.padding)) = [1; size_of::<DataWithPadding>()];
        //~^ deref_addrof
        (*(&raw mut a.tup)).1 = ();
        //~^ deref_addrof
        *(*(&raw mut a.prim)) = 0;
        //~^ deref_addrof

        (*(&raw mut a.data)).num = 42;
        (*(&raw mut a.tup)).0.num = 42;
    }
}
