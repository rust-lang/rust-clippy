//@aux-build:non-exhaustive-enum.rs
#![deny(clippy::wildcard_enum_match_arm)]
#![allow(dead_code, unreachable_code, unused_variables)]
#![allow(
    clippy::diverging_sub_expression,
    clippy::single_match,
    clippy::uninlined_format_args,
    clippy::unnested_or_patterns,
    clippy::wildcard_in_or_patterns
)]

extern crate non_exhaustive_enum;

use non_exhaustive_enum::ErrorKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
    Rgb(u8, u8, u8),
    Cyan,
}

impl Color {
    fn is_monochrome(self) -> bool {
        match self {
            Color::Red | Color::Green | Color::Blue => true,
            Color::Rgb(r, g, b) => r | g == 0 || r | b == 0 || g | b == 0,
            Color::Cyan => false,
        }
    }
}

fn main() {
    let color = Color::Rgb(0, 0, 127);
    match color {
        Color::Red => println!("Red"),
        _ => eprintln!("Not red"), //~ wildcard_enum_match_arm
    };
    match color {
        Color::Red => println!("Red"),
        _not_red => eprintln!("Not red"),
        //~^ wildcard_enum_match_arm
    };
    let _str = match color {
        Color::Red => "Red".to_owned(),
        not_red => format!("{:?}", not_red),
        //~^ wildcard_enum_match_arm
    };
    match color {
        Color::Red => {},
        Color::Green => {},
        Color::Blue => {},
        Color::Cyan => {},
        c if c.is_monochrome() => {},
        Color::Rgb(_, _, _) => {},
    };
    let _str = match color {
        Color::Red => "Red",
        c @ Color::Green | c @ Color::Blue | c @ Color::Rgb(_, _, _) | c @ Color::Cyan => "Not red",
    };
    match color {
        Color::Rgb(r, _, _) if r > 0 => "Some red",
        _ => "No red", //~ wildcard_enum_match_arm
    };
    match color {
        Color::Red | Color::Green | Color::Blue | Color::Cyan => {},
        Color::Rgb(..) => {},
    };
    let x: u8 = unimplemented!();
    match x {
        0 => {},
        140 => {},
        _ => {},
    };
    // We need to use an enum not defined in this test because non_exhaustive is ignored for the
    // purposes of dead code analysis within a crate.
    let error_kind = ErrorKind::NotFound;
    match error_kind {
        ErrorKind::NotFound => {},
        _ => {}, //~ wildcard_enum_match_arm
    }
    match error_kind {
        ErrorKind::NotFound => {},
        ErrorKind::PermissionDenied => {},
        _ => {},
    }

    {
        #![allow(clippy::manual_non_exhaustive)]
        pub enum Enum {
            A,
            B,
            #[doc(hidden)]
            __Private,
        }
        match Enum::A {
            Enum::A => (),
            _ => (), //~ wildcard_enum_match_arm
        }
    }
}
