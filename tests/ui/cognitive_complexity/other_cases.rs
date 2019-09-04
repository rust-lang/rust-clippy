#![allow(clippy::all)]
#![warn(clippy::cognitive_complexity)]
#![allow(unused)]

fn main() {}

/*
    Other Tests
*/

#[clippy::cognitive_complexity = "1"]
fn nested_functions_are_counted(n: usize) -> usize {
    fn lucas_number(n: usize) -> usize {
        match n {
            0 => 2,
            1 => 1,
            _ => lucas_number(n - 1) + lucas_number(n - 2),
        }
    }
    lucas_number(n)
}
