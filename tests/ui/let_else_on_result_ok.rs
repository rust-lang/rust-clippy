//@aux-build:proc_macros.rs
#![allow(clippy::redundant_pattern_matching, unused)]
#![warn(clippy::let_else_on_result_ok)]

#[macro_use]
extern crate proc_macros;

struct A(Result<&'static A, ()>);

enum AnEnum {
    A(Result<&'static A, ()>),
}

fn a() -> Result<(), ()> {
    Ok(())
}

fn a_constructor() -> A {
    todo!();
}

fn an_enum_constructor() -> AnEnum {
    todo!();
}

fn main() {
    // Lint
    let Ok(_) = a() else {
        return;
    };
    let (Ok(_), true) = (a(), true) else {
        return;
    };
    let [Ok(_), Ok(_)] = [a(), Err(())] else {
        return;
    };
    let A(Ok(A(Ok(A(Ok(A(Ok(_)))))))) = a_constructor() else {
        return;
    };
    let AnEnum::A(Ok(A(Err(_)))) = an_enum_constructor() else {
        return;
    };
    // Don't lint
    let Err(_) = a() else {
        return;
    };
    match a() {
        Ok(a) => a,
        Err(e) => eprintln!("{e:#?}"),
    };
    external! {
        let Ok(_) = a() else {
            return;
        };
    }
}
