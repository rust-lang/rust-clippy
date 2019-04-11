#![allow(clippy::all)]
#![warn(clippy::cognitive_complexity)]
#![allow(unused)]

fn main() {}

/*
    A collection of functions that should
    mark no score.
*/

#[clippy::cognitive_complexity = "1"]
fn chaining_with_one_block(a: bool, b: bool, c: bool) -> bool {
    a ^ {b && c}
}

/*
    Tests should never be scored
*/

// This is a normal test function

#[test]
#[clippy::cognitive_complexity = "0"]
fn simple_test(n: usize) {
    for i in range(0, n) {
        for j in range(0, n) {
            println!("{}, {}", i, j);        
        }
    }
}

// This is a test function with an inner function.
// None should be scored

#[test]
#[clippy::cognitive_complexity = "0"]
fn test_with_inner_function(n: usize) {
    fn print_and_exaggerate(n: usize) {
        println!("{}000", n);
    }

    for i in range(0, n) {
        print_and_exaggerate(n);
    }
}

/*
    Macros should not be expanded
*/

macro_rules! lucas_macro {
    ($n: expr) => {
        {
            fn lucas_number(n: usize) -> usize {
                match n {
                    0 => 2,
                    1 => 1,
                    _ => lucas_number(n - 1) + lucas_number(n - 2),
                }
            }

            lucas_number($n)
        }
    }
}

#[clippy::cognitive_complexity = "1"]
fn macros_are_not_expanded(n: usize) -> usize {
    lucas_macro!(n)
}
