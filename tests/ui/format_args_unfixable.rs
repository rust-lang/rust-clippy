#![warn(clippy::format_in_format_args, clippy::to_string_in_format_args)]
#![allow(unused)]
#![allow(clippy::assertions_on_constants, clippy::eq_op, clippy::uninlined_format_args)]

use std::io::{stdout, Error, ErrorKind, Write};
use std::ops::Deref;
use std::panic::Location;

macro_rules! my_macro {
    () => {
        // here be dragons, do not enter (or lint)
        println!("error: {}", format!("something failed at {}", Location::caller()));
    };
}

macro_rules! my_other_macro {
    () => {
        format!("something failed at {}", Location::caller())
    };
}

fn main() {
    let error = Error::new(ErrorKind::Other, "bad thing");
    let x = 'x';

    println!("error: {}", format!("something failed at {}", Location::caller()));
    println!("{}: {}", error, format!("something failed at {}", Location::caller()));
    println!("{:?}: {}", error, format!("something failed at {}", Location::caller()));
    println!("{{}}: {}", format!("something failed at {}", Location::caller()));
    println!(r#"error: "{}""#, format!("something failed at {}", Location::caller()));
    println!("error: {}", format!(r#"something failed at "{}""#, Location::caller()));
    println!("error: {}", format!("something failed at {} {0}", Location::caller()));
    let _ = format!("error: {}", format!("something failed at {}", Location::caller()));
    let _ = write!(
        stdout(),
        "error: {}",
        format!("something failed at {}", Location::caller())
    );
    let _ = writeln!(
        stdout(),
        "error: {}",
        format!("something failed at {}", Location::caller())
    );
    print!("error: {}", format!("something failed at {}", Location::caller()));
    eprint!("error: {}", format!("something failed at {}", Location::caller()));
    eprintln!("error: {}", format!("something failed at {}", Location::caller()));
    let _ = format_args!("error: {}", format!("something failed at {}", Location::caller()));
    assert!(true, "error: {}", format!("something failed at {}", Location::caller()));
    assert_eq!(0, 0, "error: {}", format!("something failed at {}", Location::caller()));
    assert_ne!(0, 0, "error: {}", format!("something failed at {}", Location::caller()));
    panic!("error: {}", format!("something failed at {}", Location::caller()));

    // negative tests
    println!("error: {}", format_args!("something failed at {}", Location::caller()));
    println!("error: {:>70}", format!("something failed at {}", Location::caller()));
    println!("error: {} {0}", format!("something failed at {}", Location::caller()));
    println!("{} and again {0}", format!("hi {}", x));
    my_macro!();
    println!("error: {}", my_other_macro!());
}

macro_rules! _internal {
    ($($args:tt)*) => {
        println!("{}", format_args!($($args)*))
    };
}

macro_rules! my_println2 {
   ($target:expr, $($args:tt)+) => {{
       if $target {
           _internal!($($args)+)
       }
    }};
}

macro_rules! my_println2_args {
    ($target:expr, $($args:tt)+) => {{
       if $target {
           _internal!("foo: {}", format_args!($($args)+))
       }
    }};
}

fn test2() {
    let error = Error::new(ErrorKind::Other, "bad thing");

    // None of these should be linted without the config change
    my_println2!(true, "error: {}", format!("something failed at {}", Location::caller()));
    my_println2!(
        true,
        "{}: {}",
        error,
        format!("something failed at {}", Location::caller())
    );

    my_println2_args!(true, "error: {}", format!("something failed at {}", Location::caller()));
    my_println2_args!(
        true,
        "{}: {}",
        error,
        format!("something failed at {}", Location::caller())
    );
}
