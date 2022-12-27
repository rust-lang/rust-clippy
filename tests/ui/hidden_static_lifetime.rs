#![allow(unused)]
#![warn(clippy::hidden_static_lifetime)]
#![allow(clippy::needless_lifetimes)]

struct S<'s> {
    s: &'s u32,
}

// Should warn

fn a<'a>() -> &'a str {
    ""
}
fn h<'h>() -> S<'h> {
    S { s: &1 }
}

// Should not warn
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

fn main() {}
