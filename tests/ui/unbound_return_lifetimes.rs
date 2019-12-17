#![allow(unused, dead_code, clippy::needless_lifetimes)]
#![warn(clippy::unbound_return_lifetimes)]

use std::collections::HashMap;
use std::hash::Hash;

struct FooStr(str);

fn unbound_fun<'a>(x: impl AsRef<str> + 'a) -> &'a FooStr {
    let x = x.as_ref();
    unsafe { &*(x as *const str as *const FooStr) }
}

fn bound_fun<'a>(x: &'a str) -> &'a FooStr {
    unsafe { &*(x as *const str as *const FooStr) }
}

fn bound_fun2<'a, 'b: 'a, S: 'b>(s: &'a S) -> &'b str {
    unreachable!()
}

type BarType<'a, T> = Bar<'a, &'a T>;

struct Bar<'a, T> {
    baz: &'a T,
}

impl<'a, T> BarType<'a, T> {
    fn bound_impl_fun(&self, _f: bool) -> &'a T {
        unreachable!()
    }
}

pub struct BazMap<'a, K, V, W> {
    alloc: &'a Vec<V>,
    map: HashMap<K, W>,
}

pub type BazRefMap<'a, K, V> = BazMap<'a, K, V, &'a V>;

impl<'a, K, V> BazRefMap<'a, K, V>
where
    K: Hash + PartialEq,
{
    pub fn or_insert(&self, key: K, value: V) -> &'a V {
        unreachable!()
    }
}

struct Quux<'a> {
    x: &'a str,
}

fn quux<'b>(q: &Quux<'b>) -> &'b str {
    unreachable!()
}

struct Quuz<T> {
    x: T,
}

impl<T> Quuz<T> {
    fn into_inner(self) -> T {
        self.x
    }
}

impl<'a> Into<&'a str> for Quuz<&'a str> {
    fn into(self) -> &'a str {
        self.into_inner()
    }
}

fn main() {}
