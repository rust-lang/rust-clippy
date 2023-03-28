#![warn(clippy::format_in_format_args, clippy::to_string_in_format_args)]
#![allow(clippy::assertions_on_constants, clippy::eq_op, clippy::uninlined_format_args)]

use std::io::{stdout, Error, ErrorKind, Write};
use std::ops::Deref;
use std::panic::Location;

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

fn main() {
    let error = Error::new(ErrorKind::Other, "bad thing");

    my_println2!(true, "error: {}", format!("something failed at {}", Location::caller()));
    my_println2!(
        true,
        "{}: {}",
        error,
        format!("something failed at {}", Location::caller())
    );

    my_println2!(
        true,
        "error: {}",
        format!("something failed at {}", Location::caller().to_string())
    );
    my_println2!(
        true,
        "{}: {}",
        error,
        format!("something failed at {}", Location::caller().to_string())
    );

    my_println2!(true, "error: {}", Location::caller().to_string());
    my_println2!(true, "{}: {}", error, Location::caller().to_string());

    my_println2_args!(true, "error: {}", format!("something failed at {}", Location::caller()));
    my_println2_args!(
        true,
        "{}: {}",
        error,
        format!("something failed at {}", Location::caller())
    );
}
