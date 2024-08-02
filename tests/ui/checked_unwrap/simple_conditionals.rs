//@no-rustfix: overlapping suggestions
#![deny(clippy::panicking_unwrap, clippy::unnecessary_unwrap)]
#![allow(
    clippy::if_same_then_else,
    clippy::branches_sharing_code,
    clippy::unnecessary_literal_unwrap
)]

macro_rules! m {
    ($a:expr) => {
        if $a.is_some() {
            // unnecessary
            $a.unwrap();
        }
    };
}

macro_rules! checks_in_param {
    ($a:expr, $b:expr) => {
        if $a {
            $b;
        }
    };
}

macro_rules! checks_unwrap {
    ($a:expr, $b:expr) => {
        if $a.is_some() {
            $b;
        }
    };
}

macro_rules! checks_some {
    ($a:expr, $b:expr) => {
        if $a {
            $b.unwrap();
        }
    };
}

fn main() {
    let x = Some(());
    if x.is_some() {
        // unnecessary
        x.unwrap();

        // unnecessary
        x.expect("an error message");

    } else {
        // will panic
        x.unwrap();

        // will panic
        x.expect("an error message");

    }
    if x.is_none() {
        // will panic
        x.unwrap();

    } else {
        // unnecessary
        x.unwrap();

    }
    m!(x);
    // ok
    checks_in_param!(x.is_some(), x.unwrap());
    // ok
    checks_unwrap!(x, x.unwrap());
    // ok
    checks_some!(x.is_some(), x);
    let mut x: Result<(), ()> = Ok(());
    if x.is_ok() {
        // unnecessary
        x.unwrap();

        // unnecessary
        x.expect("an error message");

        // will panic
        x.unwrap_err();

    } else {
        // will panic
        x.unwrap();

        // will panic
        x.expect("an error message");

        // unnecessary
        x.unwrap_err();

    }
    if x.is_err() {
        // will panic
        x.unwrap();

        // unnecessary
        x.unwrap_err();

    } else {
        // unnecessary
        x.unwrap();

        // will panic
        x.unwrap_err();

    }
    if x.is_ok() {
        x = Err(());
        // not unnecessary because of mutation of x
        // it will always panic but the lint is not smart enough to see this (it only
        // checks if conditions).
        x.unwrap();
    } else {
        x = Ok(());
        // not unnecessary because of mutation of x
        // it will always panic but the lint is not smart enough to see this (it
        // only checks if conditions).
        x.unwrap_err();
    }

    // ok, it's a common test pattern
    assert!(x.is_ok(), "{:?}", x.unwrap_err());
}

fn issue11371() {
    let option = Some(());

    if option.is_some() {
        option.as_ref().unwrap();

    } else {
        option.as_ref().unwrap();

    }

    let result = Ok::<(), ()>(());

    if result.is_ok() {
        result.as_ref().unwrap();

    } else {
        result.as_ref().unwrap();

    }

    let mut option = Some(());
    if option.is_some() {
        option.as_mut().unwrap();

    } else {
        option.as_mut().unwrap();

    }

    let mut result = Ok::<(), ()>(());
    if result.is_ok() {
        result.as_mut().unwrap();

    } else {
        result.as_mut().unwrap();

    }

    // This should not lint. Statics are, at the time of writing, not linted on anyway,
    // but if at some point they are supported by this lint, it should correctly see that
    // `X` is being mutated and not suggest `if let Some(..) = X {}`
    static mut X: Option<i32> = Some(123);
    unsafe {
        if X.is_some() {
            X = None;
            X.unwrap();
        }
    }
}

fn check_expect() {
    let x = Some(());
    if x.is_some() {
        #[expect(clippy::unnecessary_unwrap)]
        // unnecessary
        x.unwrap();
        #[expect(clippy::unnecessary_unwrap)]
        // unnecessary
        x.expect("an error message");
    } else {
        #[expect(clippy::panicking_unwrap)]
        // will panic
        x.unwrap();
        #[expect(clippy::panicking_unwrap)]
        // will panic
        x.expect("an error message");
    }
}
