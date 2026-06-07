#![warn(clippy::disallowed_modules)]

extern crate syn;

use std::sync as foo;
//~^ disallowed_modules
use std::sync::Mutex;
//~^ disallowed_modules
use std::sync::Mutex as Sneaky;
//~^ disallowed_modules
use std::net::*;
//~^ disallowed_modules
use syn::token::Token;
//~^ disallowed_modules

type DisallowedAlias = std::collections::BTreeMap<u32, u32>;
//~^ disallowed_modules

struct MyStruct(std::net::Ipv4Addr);
//~^ disallowed_modules

struct Foo {
    v1: Sneaky<usize>,
    //~^ disallowed_modules
}

enum Bar {
    V1,
    V2(std::collections::HashSet<i32>),
    //~^ disallowed_modules
}

impl std::io::Write for MyStruct {
    //~^ disallowed_modules
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        //~^ disallowed_modules
        todo!()
    }
    fn flush(&mut self) -> std::io::Result<()> {
        //~^ disallowed_modules
        todo!()
    }
}

fn generic_where<T>()
where
    T: std::io::Read,
    //~^ disallowed_modules
{
}

fn return_impl_trait() -> impl std::io::Read {
    //~^ disallowed_modules
    std::io::empty()
    //~^ disallowed_modules
}

fn bad_return_type<T>() -> fn() -> Sneaky<T> {
    //~^ disallowed_modules
    todo!()
}

fn bad_arg_type<T>(_: impl Fn(Sneaky<T>) -> foo::atomic::AtomicU32) {}
//~^ disallowed_modules
//~| disallowed_modules

fn trait_obj(_: &dyn std::io::Read) {}
//~^ disallowed_modules

static BAD: foo::atomic::AtomicPtr<()> = foo::atomic::AtomicPtr::new(std::ptr::null_mut());
//~^ disallowed_modules
//~| disallowed_modules

fn ip(_: std::net::Ipv4Addr) {}
//~^ disallowed_modules

#[allow(clippy::diverging_sub_expression)]
fn main() {
    // Original Expression Edge Cases
    let _: std::collections::HashMap<(), ()> = std::collections::HashMap::new();
    //~^ disallowed_modules
    //~| disallowed_modules
    let _ = Sneaky::new(0);
    //~^ disallowed_modules
    let _ = foo::atomic::AtomicU32::new(0);
    //~^ disallowed_modules
    static FOO: std::sync::atomic::AtomicU32 = foo::atomic::AtomicU32::new(1);
    //~^ disallowed_modules
    //~| disallowed_modules
    let _: std::collections::BTreeMap<(), usize> = Default::default();
    //~^ disallowed_modules

    let _ = <std::collections::HashMap<(), ()>>::with_capacity(0);
    //~^ disallowed_modules

    let _ = std::net::Shutdown::Both;
    //~^ disallowed_modules

    #[allow(clippy::single_match)]
    match Some(std::net::Shutdown::Both) {
        //~^ disallowed_modules
        Some(std::net::Shutdown::Read) | Some(std::net::Shutdown::Write) => {},
        //~^ disallowed_modules
        //~| disallowed_modules
        _ => {},
    }

    let _ = std::ptr::null::<()>() as *const std::sync::Mutex<()>;
    //~^ disallowed_modules

    let _ = |_: std::net::Ipv4Addr| {};
    //~^ disallowed_modules

    let v: foo::Mutex<i32>;
    //~^ disallowed_modules

    let _ = std::mem::size_of::<std::sync::Mutex<u32>>();
    //~^ disallowed_modules

    let _ = <MyStruct as std::io::Write>::write;
    //~^ disallowed_modules

    let _arr: [u8; std::mem::size_of::<std::net::Ipv4Addr>()] = [0; 4];
    //~^ disallowed_modules

    let _ = || -> Sneaky<()> {
        //~^ disallowed_modules
        todo!()
    };

    let _: Box<dyn Send + std::io::Read>;
    //~^ disallowed_modules

    if let Some(std::net::Shutdown::Read) = None
    //~^ disallowed_modules
        && let Some(std::net::Shutdown::Both) = None
    //~^ disallowed_modules
    {}
}
