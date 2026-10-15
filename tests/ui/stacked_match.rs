#![feature(postfix_match, type_ascription)]
#![warn(clippy::stacked_match)]
#![expect(clippy::match_single_binding)]

macro_rules! outer_match {
    () => {
        match match 1 {
            0 => 1,
            _ => 2,
        } {
            1 => println!("macro outer"),
            _ => println!("other"),
        }
    };
}

macro_rules! inner_match {
    () => {
        match 1 {
            0 => 1,
            _ => 2,
        }
    };
}

mod main {
    fn direct(value: u8) {
        match match value {
            //~^ stacked_match
            0 => 1,
            _ => 2,
        } {
            1 => println!("one"),
            _ => println!("other"),
        }
    }

    fn borrowing(option: Option<String>) {
        // The inner `match` borrows from a scrutinee temporary, so hoisting only the inner `match`
        // into a `let` would drop the temporary too early; the scrutinee must be bound to a local
        // first.
        match match Some(String::from("hello")) {
            //~^ stacked_match
            Some(ref text) => text,
            None => return,
        } {
            text => println!("{text}"),
        }

        // Borrowing from a place rather than a temporary; hoisting is fine here.
        match match option {
            //~^ stacked_match
            Some(ref text) => text,
            None => return,
        } {
            text => println!("{text}"),
        }
    }

    fn leftmost_operand(value: u8) {
        match match value {
            //~^ stacked_match
            0 => 1,
            _ => 2,
        } + 1
        {
            1 => println!("one"),
            _ => println!("other"),
        }

        match match value {
            //~^ stacked_match
            0 => 1u32,
            _ => 2,
        }
        .pow(2)
        {
            1 => println!("one"),
            _ => println!("other"),
        }
    }

    fn not_leftmost(value: u8) {
        match 1 + match value {
            0 => 1,
            _ => 2,
        } {
            1 => println!("one"),
            _ => println!("other"),
        }
    }

    fn not_looked_through(value: u8) {
        match (match value {
            0 => 1,
            _ => 2,
        }) {
            1 => println!("parenthesized match"),
            _ => println!("other"),
        }

        match type_ascribe!(
            match value {
                0 => 1,
                _ => 2,
            },
            i32
        ) {
            1 => println!("match in type ascription"),
            _ => println!("other"),
        }

        match if value == 0 { 1 } else { 2 } {
            1 => println!("match if"),
            _ => println!("other"),
        }
    }

    fn postfix(value: u8) {
        match value.match {
            0 => 1,
            _ => 2,
        } {
            1 => println!("postfix inner match"),
            _ => println!("other"),
        }

        value.match {
            0 => 1,
            _ => 2,
        }.match {
            1 => println!("postfix outer match"),
            _ => println!("other"),
        };
    }

    fn macros() {
        outer_match!();

        match inner_match!() {
            1 => println!("macro scrutinee"),
            _ => println!("other"),
        }
    }
}

fn main() {}
