#![warn(clippy::match_single_binding)]
#![allow(
    unused,
    clippy::let_unit_value,
    clippy::no_effect,
    clippy::toplevel_ref_arg,
    clippy::uninlined_format_args,
    clippy::useless_vec
)]

struct Point {
    x: i32,
    y: i32,
}

fn coords() -> Point {
    Point { x: 1, y: 2 }
}

macro_rules! foo {
    ($param:expr) => {
        match $param {
            _ => println!("whatever"),
        }
    };
}

fn main() {
    let a = 1;
    let b = 2;
    let c = 3;
    // Lint
    //~v match_single_binding
    match (a, b, c) {
        (x, y, z) => {
            println!("{} {} {}", x, y, z);
        },
    }
    // Lint
    //~v match_single_binding
    match (a, b, c) {
        (x, y, z) => println!("{} {} {}", x, y, z),
    }
    // Ok
    foo!(a);
    // Ok
    match a {
        2 => println!("2"),
        _ => println!("Not 2"),
    }
    // Ok
    let d = Some(5);
    match d {
        Some(d) => println!("{}", d),
        _ => println!("None"),
    }
    // Lint
    //~v match_single_binding
    match a {
        _ => println!("whatever"),
    }
    // Lint
    //~v match_single_binding
    match a {
        _ => {
            let x = 29;
            println!("x has a value of {}", x);
        },
    }
    // Lint
    //~v match_single_binding
    match a {
        _ => {
            let e = 5 * a;
            if e >= 5 {
                println!("e is superior to 5");
            }
        },
    }
    // Lint
    let p = Point { x: 0, y: 7 };
    //~v match_single_binding
    match p {
        Point { x, y } => println!("Coords: ({}, {})", x, y),
    }
    // Lint
    //~v match_single_binding
    match p {
        Point { x: x1, y: y1 } => println!("Coords: ({}, {})", x1, y1),
    }
    // Lint
    let x = 5;
    //~v match_single_binding
    match x {
        ref r => println!("Got a reference to {}", r),
    }
    // Lint
    let mut x = 5;
    //~v match_single_binding
    match x {
        ref mut mr => println!("Got a mutable reference to {}", mr),
    }
    // Lint
    //~v match_single_binding
    let product = match coords() {
        Point { x, y } => x * y,
    };
    // Lint
    let v = vec![Some(1), Some(2), Some(3), Some(4)];
    #[allow(clippy::let_and_return)]
    let _ = v
        .iter()
        //~v match_single_binding
        .map(|i| match i.unwrap() {
            unwrapped => unwrapped,
        })
        .collect::<Vec<u8>>();
    // Ok
    let x = 1;
    match x {
        #[cfg(disabled_feature)]
        0 => println!("Disabled branch"),
        _ => println!("Enabled branch"),
    }

    // Ok
    let x = 1;
    let y = 1;
    match match y {
        0 => 1,
        _ => 2,
    } {
        #[cfg(disabled_feature)]
        0 => println!("Array index start"),
        _ => println!("Not an array index start"),
    }

    // Lint
    let x = 1;
    //~v match_single_binding
    match x {
        // =>
        _ => println!("Not an array index start"),
    }
}

fn issue_8723() {
    let (mut val, idx) = ("a b", 1);

    //~v match_single_binding
    val = match val.split_at(idx) {
        (pre, suf) => {
            println!("{}", pre);
            suf
        },
    };

    let _ = val;
}

fn side_effects() {}

fn issue_9575() {
    //~v match_single_binding
    let _ = || match side_effects() {
        _ => println!("Needs curlies"),
    };
}

fn issue_9725(r: Option<u32>) {
    //~v match_single_binding
    match r {
        x => match x {
            Some(_) => {
                println!("Some");
            },
            None => {
                println!("None");
            },
        },
    };
}

fn issue_10447() -> usize {
    //~v match_single_binding
    match 1 {
        _ => (),
    }

    //~v match_single_binding
    let a = match 1 {
        _ => (),
    };

    //~v match_single_binding
    match 1 {
        _ => side_effects(),
    }

    //~v match_single_binding
    let b = match 1 {
        _ => side_effects(),
    };

    //~v match_single_binding
    match 1 {
        _ => println!("1"),
    }

    //~v match_single_binding
    let c = match 1 {
        _ => println!("1"),
    };

    let in_expr = [
        //~v match_single_binding
        match 1 {
            _ => (),
        },
        //~v match_single_binding
        match 1 {
            _ => side_effects(),
        },
        //~v match_single_binding
        match 1 {
            _ => println!("1"),
        },
    ];

    2
}
