//@revisions: edition2018 edition2021
//@[edition2018] edition:2018
//@[edition2021] edition:2021

#![warn(clippy::uninlined_format_args)]

fn main() {
    let var = 1;

    println!("val='{}'", var); //~ uninlined_format_args

    if var > 0 {
        panic!("p1 {}", var); //~[edition2021] uninlined_format_args
    }
    if var > 0 {
        panic!("p2 {0}", var); //~[edition2021] uninlined_format_args
    }
    if var > 0 {
        panic!("p3 {var}", var = var);
        //~[edition2021]^ uninlined_format_args
    }

    #[allow(non_fmt_panics)]
    {
        if var > 0 {
            panic!("p4 {var}");
        }
    }

    assert!(var == 1, "p5 {}", var);
    //~[edition2021]^ uninlined_format_args
    debug_assert!(var == 1, "p6 {}", var);
    //~[edition2021]^ uninlined_format_args
}
