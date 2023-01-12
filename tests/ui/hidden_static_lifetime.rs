#![allow(unused)]
#![warn(clippy::hidden_static_lifetime)]
#![allow(clippy::needless_lifetimes)]

struct S<'s> {
    s: &'s u32,
}

mod module {
    pub struct S<'s> {
        s: &'s u32,
    }
}

// ============= Should warn =============

fn a<'a>() -> &'a str {
    ""
}
fn h<'h>() -> S<'h> {
    S { s: &1 }
}

// Valid
fn o<'o, T>() -> &'o mut T
where
    T: 'static,
{
    unsafe { std::ptr::null::<&mut T>().read() }
}

// Only 'm1
fn n<'m1, 'm2, T>() -> &'m1 fn(&'m2 T) {
    unsafe { std::ptr::null::<&'m1 fn(&'m2 T)>().read() }
}

// ============= Should not warn =============
fn b<'b>(_: &'b str) -> &'b str {
    ""
}
fn d<'d>(_: &'d str) {}
fn e<'e>(_: &'e str) -> &'e str {
    ""
}
fn f<'f, F>(_: F) -> F
where
    F: 'f,
{
    todo!()
}
fn g<'g>(_: S<'g>) -> S<'g> {
    S { s: &1 }
}

fn i() -> S<'static> {
    S { s: &1 }
}

fn j<'j>(s: &module::S<'j>) -> S<'j> {
    S { s: &1 }
}

// Invalid
fn k<'k1, 'k2, T>() -> &'k1 mut &'k2 T {
    unsafe { std::ptr::null::<&mut &T>().read() }
}

// Invalid
fn m<'m, T>() -> fn(&'m T) {
    unsafe { std::ptr::null::<fn(&'m T)>().read() }
}

// Valid
fn l<'l, T>() -> &'l mut T
where
    T: 'static,
{
    unsafe { std::ptr::null::<&mut T>().read() }
}

fn main() {}
