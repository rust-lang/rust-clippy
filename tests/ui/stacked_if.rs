#![feature(type_ascription)]
#![warn(clippy::stacked_if)]
#![expect(clippy::blocks_in_conditions, clippy::needless_bool, clippy::redundant_closure_call)]

macro_rules! outer_if {
    () => {
        if if true { false } else { true } {
            println!("macro outer");
        }
    };
}

macro_rules! inner_if {
    () => {
        if true { false } else { true }
    };
}

mod main {
    fn direct(first: bool, second: bool, third: bool) {
        if if first {
            //~^ stacked_if
            second
        } else {
            third
        } {
            println!("stacked if");
        }
    }

    fn leftmost_operand(first: bool, second: bool, third: bool, fourth: bool) {
        if if first { second } else { third } && fourth {
            //~^ stacked_if
            println!("left-hand side of `&&`");
        }

        if if first { 1 } else { 2 } == 1 {
            //~^ stacked_if
            println!("non-boolean inner if");
        }

        if if first { 1u32 } else { 2 }.pow(2) == 1 {
            //~^ stacked_if
            println!("method call on inner if");
        }
    }

    fn if_let(option: Option<i32>) {
        if if let Some(number) = option {
            //~^ stacked_if
            number > 0
        } else {
            false
        } {
            println!("if let");
        }
    }

    fn triple(first: bool, second: bool, third: bool) {
        if if if first {
            //~^ stacked_if
            //~^^ stacked_if
            second
        } else {
            third
        } {
            second
        } else {
            third
        } {
            println!("triple if");
        }
    }

    const STACKED_IN_CONST: bool = if if true { false } else { true } {
        //~^ stacked_if
        true
    } else {
        false
    };

    fn not_leftmost(first: bool, second: bool, third: bool, fourth: bool, option: Option<i32>) {
        if fourth == if first { second } else { third } && second {
            println!("right-hand side of `==`");
        }

        if first && second && third && fourth && if first { second } else { third } {
            println!("long binary condition");
        }

        if let Some(number) = option
            && if number > 0 { true } else { false }
        {
            println!("let chain");
        }
    }

    fn not_looked_through(first: bool, second: bool, third: bool, option: Option<i32>) {
        if (if first { second } else { third }) {
            println!("parenthesized if");
        }

        if { if first { second } else { third } } {
            println!("if in block");
        }

        if (|| if first { second } else { third })() {
            println!("if in closure");
        }

        if bool_identity(if first { second } else { third }) {
            println!("if in call");
        }

        if type_ascribe!(if first { second } else { third }, bool) {
            println!("if in type ascription");
        }

        if match option {
            Some(_) => first,
            None => second,
        } {
            println!("if match");
        }
    }

    fn macros() {
        outer_if!();

        if inner_if!() {
            println!("macro condition");
        }
    }

    fn bool_identity(value: bool) -> bool {
        value
    }
}

fn main() {}
