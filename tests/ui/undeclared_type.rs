#![warn(clippy::undeclared_type)]

fn get_value() -> Vec<i32> {
    vec![]
}

fn main() {
    // Warning
    let a = get_value();
    //~^ undeclared_type

    // No warning
    let b: Vec<i32> = get_value();
}
