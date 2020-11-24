#![warn(clippy::map_err_ignore)]
#![allow(clippy::unnecessary_wraps)]
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
enum Errors {
    Ignored,
    OwnContext(u32),
    WithContext(std::num::TryFromIntError),
}

impl Error for Errors {}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error")
    }
}

#[derive(Debug)]
struct SError {
    x: u32,
}

fn test_fn_call() -> u32 {
    0
}

fn main() -> Result<(), Errors> {
    let x = u32::try_from(-123_i32);
    let y = 0;

    // Should not warn you here, because you are giving context with a non-unit enum variant
    println!("{:?}", x.map_err(|_| Errors::OwnContext(0)));

    // Should not warn you here, because you are wrapping the context
    println!("{:?}", x.map_err(Errors::WithContext));

    // Should not warn you here, because you are giving context via a `String`
    println!("{:?}", x.map_err(|_| "There was an error!"));

    // Should not warn you here, because you are calling a constant for context
    println!("{:?}", x.map_err(|_| u32::MAX));

    // Should not warn you here, because you are calling a function for context
    println!("{:?}", x.map_err(|_| test_fn_call()));

    // Should not warn you here, because you are providing a variable for context
    println!("{:?}", x.map_err(|_| y));

    // Should not warn you here, because you are providing a struct for context
    println!("{:?}", x.map_err(|_| SError { x: 0 }));

    // Should warn you here because you are just ignoring the original error
    println!("{:?}", x.map_err(|_| Errors::Ignored));

    // Should not warn you because you explicitly ignore the parameter
    println!("{:?}", x.map_err(|_ignored_no_extra_context| Errors::Ignored));

    Ok(())
}
