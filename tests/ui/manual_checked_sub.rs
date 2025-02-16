#![allow(clippy::collapsible_if)]
#![warn(clippy::manual_checked_sub)]

//Positive test, lint
fn positive_tests() {
    let a: u32 = 10;
    let b: u32 = 5;

    // Case 1: a - b inside an if a >= b
    if a >= b {
        let _ = a - b;
    }

    // Case 2: a - 1 inside an if a > 0
    if a > 0 {
        let _ = a - 1;
    }

    // Case 3: Both suffixed, should lint
    if 10u32 >= 5u32 {
        let _ = 10u32 - 5u32;
    }

    let x: i32 = 10;
    let y: i32 = 5;

    // Case 4: casted variables inside an if, should lint
    if x as u32 >= y as u32 {
        let _ = x as u32 - y as u32;
    }

    // Case 5: casted variable and literal inside an if, should lint
    if x as u32 >= 5u32 {
        let _ = x as u32 - 5u32;
    }
}

// Negative tests, no lint
fn negative_tests() {
    let a: u32 = 10;
    let b: u32 = 5;

    // Case 1: Uses checked_sub() correctly, no lint
    let _ = a.checked_sub(b);

    // Case 2: Works with signed integers (i32), no lint
    let x: i32 = 10;
    let y: i32 = 5;
    if x >= y {
        let _ = x - y;
    }

    // Case 3: If condition doesn't match, no lint
    if a == b {
        let _ = a - b;
    }

    // Case 4: a - b inside an if a > b
    if a > b {
        let _ = a - b;
    }

    // Case 5: Unsuffixed literals, no lint
    if 10 >= 5 {
        let _ = 10 - 5;
    }

    // Case 6: Suffixed lhs, unsuffixed rhs, no lint
    if 10u32 >= 5 {
        let _ = 10u32 - 5;
    }

    // Case 7: Unsuffixed lhs, suffixed rhs, no lint
    if 10 >= 5u32 {
        let _ = 10 - 5u32;
    }
}

fn edge_cases() {
    let a: u32 = 10;
    let b: u32 = 5;
    let c = 3;

    // Edge Case 1: Subtraction inside a nested if, should lint
    if a >= b {
        if c > 0 {
            let _ = a - b;
        }
    }

    // no lint
    if a > b {
        if a - b > c {
            let _ = a - b;
        }
    }
}

fn main() {
    positive_tests();
    negative_tests();
    edge_cases();
}
