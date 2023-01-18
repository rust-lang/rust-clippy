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

struct SMut<'a>(&'a mut i32);
struct STuple<'a>(&'a i32);
struct SGeneric<T>(T);
struct SRef<'a, T: 'a>(&'a T);
struct Both<'a, 'b, T> {
    owned: T,
    borrow: &'a T,
    mut_borrow: &'b mut T,
}

// ============= Should warn =============

fn a<'a>() -> &'a str {
    ""
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

// Only 's1
fn s<'s1, 's2>() -> &'s1 STuple<'s2> {
    unsafe { std::ptr::null::<&STuple<'s2>>().read() }
}

fn q<'q>() -> STuple<'q> {
    STuple(&1)
}

// Only 'r1
fn r<'r1, 'r2>() -> &'r1 SMut<'r2> {
    unsafe { std::ptr::null::<&SMut<'r2>>().read() }
}

// ============= Should not warn =============
fn b<'b>(_: &'b str) -> &'b str {
    ""
}
fn d<'d>(_: &'d str) {}
fn e<'e>(_: &'e str) -> &'e str {
    ""
}

fn h<'h>() -> S<'h> {
    S { s: &1 }
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

fn p<'p>() -> SMut<'p> {
    unsafe { std::ptr::null::<SMut<'p>>().read() }
}

fn t<'t1, 't2>() -> SGeneric<&'t1 mut SRef<'t2, u32>> {
    unsafe { std::ptr::null::<SGeneric<&mut SRef<'t2, u32>>>().read() }
}

fn u<'u1, 'u2>() -> &'u1 Both<'u1, 'u2, &'u1 str> {
    unsafe { std::ptr::null::<&Both<'u1, 'u2, &'u1 str>>().read() }
}

fn v<'v1, 'v2, 'v3>() -> &'v1 Both<'v1, 'v2, &'v3 str> {
    unsafe { std::ptr::null::<&Both<'v1, 'v2, &'v3 str>>().read() }
}

fn w<'w1, 'w2>() -> &'w1 Both<'w1, 'w2, &'static str> {
    unsafe { std::ptr::null::<&Both<'w1, 'w2, &'static str>>().read() }
}

fn x<'x>() -> SRef<'x, fn(SGeneric<SRef<'x, u32>>)> {
    unsafe { std::ptr::null::<SRef<'x, fn(SGeneric<SRef<'x, u32>>)>>().read() }
}

fn main() {}
