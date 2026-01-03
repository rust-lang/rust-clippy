#![warn(clippy::filter_some)]

macro_rules! unchanged {
    ($result:expr) => {
        $result
    };
}

macro_rules! condition {
    ($condition:expr) => {
        $condition || false
    };
}

#[clippy::msrv = "1.61"]
fn older() {
    let _ = Some(0).filter(|_| false);
}

#[clippy::msrv = "1.62"]
fn newer() {
    let _ = Some(0).filter(|_| false);
    //~^ filter_some
}

fn main() {
    let _ = Some(0).filter(|_| false);
    //~^ filter_some
    // The condition contains an operator.  The program should add parentheses.
    let _ = Some(0).filter(|_| 1 == 0);
    //~^ filter_some
    let _ = Some(0).filter(|_| match 0 {
        //~^ filter_some
        0 => false,
        1 => true,
        _ => true,
    });
    // The argument to filter requires the value in the option.  The program
    // can't figure out how to change it.  It should leave it alone for now.
    let _ = Some(0).filter(|x| *x == 0);
    // The expression is a macro argument.  The program should change the macro
    // argument.  It should not expand the macro.
    let _ = unchanged!(Some(0).filter(|_| false));
    //~^ filter_some
    let _ = Some(0).filter(|_| vec![false].is_empty());
    //~^ filter_some
    // The condition is a macro that expands to an expression containing an
    // operator.  The program should not add parentheses.
    let _ = Some(0).filter(|_| condition!(false));
    //~^ filter_some
}
