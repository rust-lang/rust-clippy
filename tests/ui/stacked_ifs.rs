#![warn(clippy::stacked_ifs)]
#![allow(clippy::manual_unwrap_or_default)]

macro_rules! m1 {
    () => {
        let a = true;
        let b = false;
        if if a { b } else { true } {
            println!("True");
        }
    };
}

macro_rules! m2 {
    () => {
        if true { true } else { false }
    };
}

fn main() {
    let a = 3;
    let b = 4;
    if a == b { // No error
        // Do something.
    } else {
        // Do something else.
    }

    if if a % 2 == 0 {
        //~^ ERROR: stacked `if` found
        a - 1
    } else {
        a + 1
    } == b
    {
        // Do something.
    }

    if if a % 2 == 0 {
        //~^ ERROR: stacked `if` found
        a - 1
    } else {
        a + 1
    } == b
    {
        // Do something.
    } else {
        // Do something else.
    }

    if b == if a % 2 == 0 {
        //~^ ERROR: stacked `if` found
        a - 1
    } else {
        a + 1
    } {
        // Do something.
    } else {
        // Do something else.
    }

    let c = 5;
    let d = 6;

    // More complex statements (example from issue 12483):
    if if if a == b {
        //~^ ERROR: stacked `if` found
        //~^^ ERROR: stacked `if` found
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

    let a = true;
    let b = false;

    if if a { b } else { true } {
        //~^ ERROR: stacked `if` found
        // Do something.
    }

    let x = Some(10);

    if if let Some(num) = x { num } else { 0 } == 0 {
        //~^ ERROR: stacked `if` found
        // Do something.
    }

    m1!(); // No error

    if m2!() {
        // No error
    }
}
