#![warn(clippy::stacked_if_match)]
#![allow(unused)]

fn stacked_if() {
    let x = 0;
    if if x == 1 { x == 2 } else { x == 3 } {
        println!("true");
    }

    if if x == 1 { 2 } else { 3 } == 4 {
        println!("true");
    }

    if if x == 1 {
        let y = 2;
        y == 2
    } else {
        let z = 3;
        z == 3
    } {
        println!("true");
    }
}

fn stacked_match() {
    let x = 0;
    match match x {
        1 => 2,
        _ => 3,
    } {
        1 => {},
        2 => {},
        _ => {},
    }

    match match x {
        1 => 2,
        _ => 3,
    } + 1
    {
        1 => {},
        2 => {},
        _ => {},
    }
}

fn if_no_lint() {
    let x = 0;

    if (if x == 1 { x == 2 } else { x == 3 }) {
        println!("true");
    }

    if 1 == if x == 1 { 1 } else { 2 } {
        println!("true");
    }
}

fn match_no_lint() {
    let x = 0;
    match (match x {
        1 => 2,
        _ => 3,
    }) {
        1 => {},
        2 => {},
        _ => {},
    }

    match 1 + match x {
        1 => 2,
        _ => 3,
    } {
        1 => {},
        2 => {},
        _ => {},
    }
}

fn if_match_no_lint() {
    let x = 0;
    if match x {
        1 => 2,
        _ => 3,
    } == 4 {
        println!("true");
    }
}

fn match_if_no_lint() {
    let x = 0;
    match if x == 1 {
        let y = 2;
        y + 1
    } else {
        let z = 3;
        z + 1
    } {
        1 => {},
        2 => {},
        _ => {},
    }
}

macro_rules! if_macro {
    ($var:ident) => {
        if $var == 1 { true } else { false }
    };
}

macro_rules! match_macro {
    ($var:ident) => {
        match $var {
            1 => 2,
            _ => 3,
        }
    };
}

macro_rules! if_if_macro {
    () => {
        if if true { true } else { false } {
            println!("true");
        }
    };
}

fn macro_no_lint() {
    let x = 0;
    if if_macro!(x) {
        println!("true");
    }

    match match_macro!(x) {
        1 => {},
        2 => {},
        _ => {},
    }

    if_if_macro!();
}

fn main() {}
