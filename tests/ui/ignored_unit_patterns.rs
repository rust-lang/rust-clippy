//@aux-build:proc_macro_derive.rs
#![warn(clippy::ignored_unit_patterns)]
#![allow(
    clippy::let_unit_value,
    clippy::redundant_pattern_matching,
    clippy::single_match,
    clippy::needless_borrow
)]

fn foo() -> Result<(), ()> {
    unimplemented!()
}

fn main() {
    match foo() {
        Ok(_) => {},  //~ ERROR: matching over `()` is more explicit
        //~^ ignored_unit_patterns
        Err(_) => {}, //~ ERROR: matching over `()` is more explicit
        //~^ ignored_unit_patterns
    }
    if let Ok(_) = foo() {}
    //~^ ignored_unit_patterns
    let _ = foo().map_err(|_| todo!());
    //~^ ignored_unit_patterns

    println!(
        "{:?}",
        match foo() {
            Ok(_) => {},
            //~^ ignored_unit_patterns
            Err(_) => {},
            //~^ ignored_unit_patterns
        }
    );
}

// ignored_unit_patterns in derive macro should be ok
#[derive(proc_macro_derive::StructIgnoredUnitPattern)]
pub struct B;

#[allow(unused)]
pub fn moo(_: ()) {
    let _ = foo().unwrap();
    //~^ ignored_unit_patterns
    let _: () = foo().unwrap();
    let _: () = ();
}

fn test_unit_ref_1() {
    let x: (usize, &&&&&()) = (1, &&&&&&());
    match x {
        (1, _) => unimplemented!(),
        //~^ ignored_unit_patterns
        _ => unimplemented!(),
    };
}

fn test_unit_ref_2(v: &[(usize, ())]) {
    for (x, _) in v {
    //~^ ignored_unit_patterns
        let _ = x;
    }
}
