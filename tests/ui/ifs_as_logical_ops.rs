#![warn(clippy::ifs_as_logical_ops)]
#![allow(clippy::needless_bool)]

fn main() {
    // test code goes here
}

fn basic_lint_test(b1: bool, b2: bool) -> bool {
    if b1 { b2 } else { false }
    //~^ ifs_as_logical_ops
}

fn test_that_nested_expressions_produces_lint(b1: bool, b2: bool) -> bool {
    if b1 { { b2 } } else { false }
    //~^ ifs_as_logical_ops
}

fn test_that_diverging_does_not_produce_lint(b1: bool, b2: bool) -> bool {
    if b1 { panic!() } else { false }
}

fn test_that_complex_expressions_do_not_produce_lint(b1: bool, b2: bool) -> bool {
    if b1 {
        let mut some_value = 100;
        if some_value < 50 {
            return true;
        }
        true
    } else {
        false
    }
}

macro_rules! always_return_true {
    () => {
        true
    };
}

fn test_that_macro_expressions_produce_lint_correctly(b1: bool) -> bool {
    if b1 { always_return_true!() } else { false }
    //~^ ifs_as_logical_ops
}
