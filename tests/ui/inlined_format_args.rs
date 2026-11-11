//@aux-build:proc_macros.rs

#![warn(clippy::inlined_format_args)]
#![allow(named_arguments_used_positionally)]

extern crate proc_macros;
use proc_macros::with_span;

use std::fmt::Write as _;

macro_rules! inlined_in_macro {
    () => {
        let x = 1;
        println!("{x}");
    };
}

const CONST: i32 = 7;

fn main() {
    let local_i32 = 1;
    let local_f64 = 2.0;
    let local_opt: Option<i32> = Some(3);
    let width = 4;
    let prec = 1;

    // Simple inlined values
    println!("val='{local_i32}'");
    //~^ inlined_format_args
    println!("val='{local_i32:?}'");
    //~^ inlined_format_args
    println!("val='{local_i32:#?}'");
    //~^ inlined_format_args
    println!("val='{local_i32:4}'");
    //~^ inlined_format_args
    println!("val='{local_i32:<4}'");
    //~^ inlined_format_args
    println!("val='{local_i32:#010x}'");
    //~^ inlined_format_args
    println!("{local_opt:?}");
    //~^ inlined_format_args
    println!("{CONST}");
    //~^ inlined_format_args
    let _ = format!("{local_i32}");
    //~^ inlined_format_args
    print!("{local_i32}");
    //~^ inlined_format_args
    eprintln!("{local_i32}");
    //~^ inlined_format_args
    let mut s = String::new();
    let _ = write!(s, "{local_i32}");
    //~^ inlined_format_args
    let _ = writeln!(s, "{local_i32}");
    //~^ inlined_format_args

    // Multiple and repeated inlined values
    println!("{local_i32} {local_f64}");
    //~^ inlined_format_args
    println!("{local_i32} {local_i32}");
    //~^ inlined_format_args
    println!("{{}} {local_i32} {{local_f64}}");
    //~^ inlined_format_args

    // Mixed with explicit positional arguments
    println!("{} {local_i32}", local_f64);
    //~^ inlined_format_args
    println!("{local_i32} {}", local_f64);
    //~^ inlined_format_args
    println!("{local_i32} {} {local_opt:?} {}", local_f64, 5);
    //~^ inlined_format_args
    println!("{local_i32} {}", local_f64,);
    //~^ inlined_format_args
    println!("{local_i32} {}", local_i32 + 1);
    //~^ inlined_format_args
    println!(r"{local_i32} {}", local_f64);
    //~^ inlined_format_args
    println!(r#"{local_i32} "{}""#, local_f64);
    //~^ inlined_format_args
    println!("\n{local_i32}\t{}", local_f64);
    //~^ inlined_format_args

    // Mixed with explicit named arguments
    println!("{local_i32} {val}", val = local_f64);
    //~^ inlined_format_args
    println!("{} {local_i32} {val}", 5, val = local_f64);
    //~^ inlined_format_args

    // Inlined width and precision
    println!("{local_f64:width$}");
    //~^ inlined_format_args
    println!("{local_f64:.prec$}");
    //~^ inlined_format_args
    println!("{local_f64:width$.prec$}");
    //~^ inlined_format_args
    println!("{:width$}", local_f64);
    //~^ inlined_format_args
    println!("{:width$} {:width$}", local_f64, local_i32);
    //~^ inlined_format_args
    println!("{local_f64:.*}", 3);
    //~^ inlined_format_args
    println!("{:.*} {local_f64:.prec$}", 3, 1.5);
    //~^ inlined_format_args

    // Multiline
    println!(
        //~^ inlined_format_args
        "{}
        {local_i32}",
        local_f64,
    );

    // Nested `format_args!`
    println!("{}", format_args!("{local_i32}"));
    //~^ inlined_format_args

    // Panics and asserts
    if local_i32 > 100 {
        panic!("{local_i32}");
        //~^ inlined_format_args
    }
    assert!(local_i32 > 0, "{local_i32}");
    //~^ inlined_format_args
    assert_eq!(local_i32, 1, "{local_i32}");
    //~^ inlined_format_args

    // Don't lint: nothing is inlined
    println!("{}", local_i32);
    println!("{:?}", local_i32);
    println!("{0} {0}", local_i32);
    println!("{val}", val = local_i32);
    println!("{:1$}", local_f64, width);
    println!("{:.*}", prec, local_f64);
    println!("{{local_i32}}");
    println!("no args");
    println!();

    // Don't lint: from macros
    inlined_in_macro!();
    with_span!(span println!("{local_i32}"));
}
