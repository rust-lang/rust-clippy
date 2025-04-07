#![allow(clippy::collapsible_if)]
#![warn(clippy::manual_checked_sub)]

fn positive_tests() {
    let a: u32 = 10;
    let b: u32 = 5;
    let c: u32 = 3;

    if a >= b {
        //~^ manual_checked_sub
        let d = a - b;
    }

    if b <= a {
        //~^ manual_checked_sub
        let d = a - b;
    }

    if a >= b {
        //~^ manual_checked_sub
        let d = a - b;
    } else {
        let d = a + b;
    }

    if a >= b {
        //~^ manual_checked_sub
        let d = a - b;
    } else if a < b {
        let d = a + b;
    }

    if a > 0 {
        //~^ manual_checked_sub
        let _ = a - 1;
    }

    if 0 < a {
        //~^ manual_checked_sub
        let _ = a - 1;
    }

    if a >= b {
        //~^ manual_checked_sub
        some_function(a - b);
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

    if a >= b {
        //~^ manual_checked_sub
        if c > 0 {
            let _ = a - b;
        }
    }

    if a >= b {
        if c > 0 {
            //~^ manual_checked_sub
            let _ = c - 1;
        }
    }
}

fn some_function(a: u32) {}

fn negative_tests() {
    let a: u32 = 10;
    let b: u32 = 5;

    let x: i32 = 10;
    let y: i32 = 5;

    let mut a_mut = 10;

    if a_mut >= b {
        a_mut *= 2;
        let c = a_mut - b;
    }

    if a_mut > 0 {
        a_mut *= 2;
        let c = a_mut - 1;
    }

    if x >= y {
        let _ = x - y;
    }

    if a == b {
        let _ = a - b;
    }

    if a > b {
        let _ = a - b;
    }

    if 10 >= 5 {
        let _ = 10 - 5;
    }
}

fn main() {
    positive_tests();
    negative_tests();
}
