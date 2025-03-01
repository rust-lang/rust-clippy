#![allow(clippy::collapsible_if)]
#![warn(clippy::manual_checked_sub)]

fn positive_tests() {
    let a: u32 = 10;
    let b: u32 = 5;
    let c = 3;

    // Basic subtraction inside an if
    if a >= b {
        //~^ manual_checked_sub
        let c = a - b;
    }

    // Basic subtraction inside an if with an else block
    if a >= b {
        //~^ manual_checked_sub
        let c = a - b;
    } else {
        let c = a + b;
    }

    // Decrementing inside an if condition
    if a > 0 {
        //~^ manual_checked_sub
        let _ = a - 1;
    }

    // Variable subtraction used in a function call
    if a >= b {
        //~^ manual_checked_sub
        some_function(a - b);
    }

    // Subtracting two suffixed literals
    if 10u32 >= 5u32 {
        //~^ manual_checked_sub
        let _ = 10u32 - 5u32;
    }

    let x: i32 = 10;
    let y: i32 = 5;

    // Casted variables inside an if
    if x as u32 >= y as u32 {
        //~^ manual_checked_sub
        let _ = x as u32 - y as u32;
    }

    // Casted variable and literal inside an if
    if x as u32 >= 5u32 {
        //~^ manual_checked_sub
        let _ = x as u32 - 5u32;
    }

    // Casting subtraction result
    if a >= b {
        //~^ manual_checked_sub
        let _ = (a - b) as u64;
    }

    if a >= b {
        //~^ manual_checked_sub
        macro_rules! subtract {
            () => {
                a - b
            };
        }
        let _ = subtract!();
    }

    struct Example {
        value: u32,
    }

    if a >= b {
        //~^ manual_checked_sub
        let _ = Example { value: a - b };
    }

    // Subtraction inside a nested if, should lint
    if a >= b {
        //~^ manual_checked_sub
        if c > 0 {
            let _ = a - b;
        }
    }
}

fn some_function(a: u32) {}

fn negative_tests() {
    let a: u32 = 10;
    let b: u32 = 5;

    // Uses `.checked_sub()`, should not trigger
    let _ = a.checked_sub(b);

    // Works with signed integers (i32), should not trigger
    let x: i32 = 10;
    let y: i32 = 5;

    if x >= y {
        let _ = x - y;
    }

    // If condition does not match subtraction variables
    if a == b {
        let _ = a - b;
    }

    // a - b inside an if a > b (shouldn't lint)
    if a > b {
        let _ = a - b;
    }

    // Unsuffixed literals (shouldn't lint)
    if 10 >= 5 {
        let _ = 10 - 5;
    }

    // Suffixed LHS, unsuffixed RHS (shouldn't lint)
    if 10u32 >= 5 {
        let _ = 10u32 - 5;
    }

    //  Unsuffixed LHS, suffixed RHS (shouldn't lint)
    if 10 >= 5u32 {
        let _ = 10 - 5u32;
    }

    // Using `.wrapping_sub()`, should not trigger
    if a >= b {
        let _ = a.wrapping_sub(b);
    }
}

fn main() {
    positive_tests();
    negative_tests();
}
