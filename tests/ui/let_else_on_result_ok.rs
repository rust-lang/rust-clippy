//@aux-build:proc_macros.rs
#![allow(clippy::redundant_pattern_matching, unused)]
#![warn(clippy::let_else_on_result_ok)]

#[macro_use]
extern crate proc_macros;

struct A(Result<&'static A, ()>);

enum AnEnum {
    A(Result<&'static A, ()>),
}

struct B;

struct C(u32);

enum D {
    A,
    B,
}

enum E {
    A,
    B(u32),
}

#[repr(C)]
union F {
    a: u32,
}

#[non_exhaustive]
struct G {}

#[non_exhaustive]
enum H {}

fn a() -> Result<(), ()> {
    Ok(())
}

fn b() -> Result<(), ()> {
    let (Ok(_), 1) = (a(), 1) else {
        return Err::<(), _>(());
    };
    Ok(())
}

fn c() -> Result<(), C> {
    todo!()
}

fn d() -> Result<(), D> {
    todo!()
}

fn e() -> Result<(), E> {
    todo!()
}

fn f() -> Result<(), F> {
    todo!()
}

fn g() -> Result<(), G> {
    todo!()
}

fn h() -> Result<(), H> {
    todo!()
}

fn a_constructor() -> A {
    todo!();
}

fn an_enum_constructor() -> AnEnum {
    todo!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Lint
    let Ok(_) = c() else {
        return Ok(());
    };
    let Ok(_) = e() else {
        return Ok(());
    };
    let Ok(_) = f() else {
        return Ok(());
    };
    let Ok(_) = g() else {
        return Ok(());
    };
    let Ok(_) = h() else {
        return Ok(());
    };
    // Don't lint
    loop {
        let Ok(_) = c() else {
            continue;
        };
    }
    let Err(_) = a() else {
        return Ok(());
    };
    let Ok(_) = a() else {
        return Ok(());
    };
    let Ok(_) = b() else {
        return Ok(());
    };
    let Ok(_) = d() else {
        return Ok(());
    };
    match a() {
        Ok(a) => a,
        Err(e) => eprintln!("{e:#?}"),
    };
    external! {
        let Ok(_) = a() else {
            return Ok(());
        };
    }
    Ok(())
}

fn no_result_main() {
    // Don't lint
    let Ok(_) = c() else {
        return;
    };
    let Ok(_) = e() else {
        return;
    };
    let Ok(_) = f() else {
        return;
    };
}
