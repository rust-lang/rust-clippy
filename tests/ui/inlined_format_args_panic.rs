//@revisions: edition2018 edition2021 edition2024
//@[edition2018] edition:2018
//@[edition2021] edition:2021
//@[edition2024] edition:2024

#![warn(clippy::inlined_format_args)]
#![allow(non_fmt_panics)]

fn main() {
    let var = 1;

    println!("val='{var}'");
    //~^ inlined_format_args

    if var > 0 {
        panic!("p1 {var}");
        //~[edition2021,edition2024]^ inlined_format_args
    }
    if var > 0 {
        panic!("p2 {0}", var);
    }
    if var > 0 {
        panic!("p3 {var}", var = var);
    }

    assert!(var == 1, "p4 {var}");
    //~[edition2021,edition2024]^ inlined_format_args
    debug_assert!(var == 1, "p5 {var}");
    //~[edition2021,edition2024]^ inlined_format_args
}
