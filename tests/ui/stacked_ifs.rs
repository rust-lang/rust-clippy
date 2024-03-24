#![warn(clippy::stacked_ifs)]

fn main() {
    let a = 3;
    let b = 4;
    if a == b { // No error
        // Do something.
    } else {
        // Do something else.
    }

    if if a % 2 == 0 {
        //~^ ERROR: Stacked `if` found
        a - 1
    } else {
        a + 1
    } == b
    {
        // Do something.
    }

    if if a % 2 == 0 {
        //~^ ERROR: Stacked `if` found
        a - 1
    } else {
        a + 1
    } == b
    {
        // Do something.
    } else {
        // Do something else.
    }

    let c = 5;
    let d = 6;

    // More complex statements (example from issue 12483):
    if if if a == b {
        //~^ ERROR: Stacked `if` found
        //~^^ ERROR: Stacked `if` found
        b == c
    } else {
        a == c
    } {
        a == d
    } else {
        c == d
    } {
        println!("True!");
    } else {
        println!("False!");
    }
}
