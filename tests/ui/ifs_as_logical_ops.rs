#![warn(clippy::ifs_as_logical_ops)]
#![allow(clippy::needless_bool)]
#![allow(clippy::match_like_matches_macro)]

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

fn test_example_with_comment_does_not_lint(chars: Vec<char>) -> bool {
    if !chars.is_empty() {
        // SAFETY: Always starts with ^[ and ends with m.
        chars.len() > 5
    } else {
        false
    }
}

fn test_example_with_debug_does_not_lint(b1: bool) -> bool {
    if b1 {
        dbg!("Something");
        true
    } else {
        false
    }
}

fn test_example_with_if_let_does_not_lint(b1: bool, b2: Option<i32>) -> bool {
    if let Some(30) = b2 { true } else { false }
}

fn test_else_if_does_lint(b1: bool, b2: bool, b3: bool) -> bool {
    if b1 {
        // There is some expansion here.
        let _ = 40;
        false
    } else if b2 {
        b3
    } else {
        false
    }
    //~^^^^^ ifs_as_logical_ops
}

fn needless_bool_clash_does_not_lint(x: bool) -> bool {
    if x { true } else { false }
}

macro_rules! always_return_true {
    () => {
        true
    };
}

fn needless_bool_macro_clash_does_not_lint(b1: bool) -> bool {
    if b1 { always_return_true!() } else { false }
}

macro_rules! nothing {
    () => {};
}

fn test_empty_expansion_does_not_lint(b1: bool, b2: bool) -> bool {
    if b1 {
        nothing!();
        b2
    } else {
        false
    }
}
