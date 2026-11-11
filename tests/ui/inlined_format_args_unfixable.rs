//@no-rustfix
#![warn(clippy::inlined_format_args)]
#![allow(named_arguments_used_positionally)]

fn main() {
    let local_i32 = 1;
    let local_f64 = 2.0;
    let width = 4;

    // Arguments referred to by index would be shifted by the inserted arguments
    println!("{0} {local_i32}", local_f64);
    //~^ inlined_format_args
    println!("{local_f64:0$}", width);
    //~^ inlined_format_args
    println!("{1} {local_i32} {0}", local_f64, 5);
    //~^ inlined_format_args

    // Named argument referred to positionally
    println!("{} {local_i32}", val = local_f64);
    //~^ inlined_format_args
}
