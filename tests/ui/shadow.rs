//@aux-build:proc_macro_derive.rs

#![warn(clippy::shadow_same, clippy::shadow_reuse, clippy::shadow_unrelated)]
#![allow(
    clippy::let_unit_value,
    clippy::needless_if,
    clippy::redundant_guards,
    clippy::redundant_locals
)]

extern crate proc_macro_derive;

#[derive(proc_macro_derive::ShadowDerive)]
pub struct Nothing;

macro_rules! reuse {
    ($v:ident) => {
        let $v = $v + 1;
    };
}

fn shadow_same() {
    let x = 1;
    let x = x;
    //~^ shadow_same
    let mut x = &x;
    //~^ shadow_same
    let x = &mut x;
    //~^ shadow_same
    let x = *x;
    //~^ shadow_same
}

fn shadow_reuse() -> Option<()> {
    let x = ([[0]], ());
    let x = x.0;
    //~^ shadow_reuse
    let x = x[0];
    //~^ shadow_same
    let [x] = x;
    //~^ shadow_same
    let x = Some(x);
    //~^ shadow_same
    let x = foo(x);
    //~^ shadow_same
    let x = || x;
    //~^ shadow_same
    let x = Some(1).map(|_| x)?;
    //~^ shadow_same
    let y = 1;
    let y = match y {
    //~^ shadow_same
        1 => 2,
        _ => 3,
    };
    None
}

fn shadow_reuse_macro() {
    let x = 1;
    // this should not warn
    reuse!(x);
}

fn shadow_unrelated() {
    let x = 1;
    let x = 2;
    //~^ shadow_unrelated
}

fn syntax() {
    fn f(x: u32) {
        let x = 1;
        //~^ shadow_same
    }
    let x = 1;
    match Some(1) {
        Some(1) => {},
        Some(x) => {
        //~^ shadow_same
            let x = 1;
            //~^ shadow_same
        },
        _ => {},
    }
    if let Some(x) = Some(1) {}
    //~^ shadow_same
    while let Some(x) = Some(1) {}
    //~^ shadow_same
    let _ = |[x]: [u32; 1]| {
    //~^ shadow_same
        let x = 1;
        //~^ shadow_same
    };
    let y = Some(1);
    if let Some(y) = y {}
    //~^ shadow_same
}

fn negative() {
    match Some(1) {
        Some(x) if x == 1 => {},
        Some(x) => {},
        None => {},
    }
    match [None, Some(1)] {
        [Some(x), None] | [None, Some(x)] => {},
        _ => {},
    }
    if let Some(x) = Some(1) {
        let y = 1;
    } else {
        let x = 1;
        let y = 1;
    }
    let x = 1;
    #[allow(clippy::shadow_unrelated)]
    let x = 1;
}

fn foo<T>(_: T) {}

fn question_mark() -> Option<()> {
    let val = 1;
    // `?` expands with a `val` binding
    None?;
    None
}

pub async fn foo1(_a: i32) {}

pub async fn foo2(_a: i32, _b: i64) {
    let _b = _a;
    //~^ shadow_same
}

fn ice_8748() {
    let _ = [0; {
        let x = 1;
        if let Some(x) = Some(1) { x } else { 1 }
        //~^ shadow_same
    }];
}

fn main() {}
