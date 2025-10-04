#![warn(clippy::parsed_string_literals)]

use std::ffi::c_int;

fn main() {
    _ = "10".parse::<usize>().unwrap();
    //~^ parsed_string_literals
    _ = "1.23".parse::<f32>().unwrap();
    //~^ parsed_string_literals
    _ = "1.2300".parse::<f32>().unwrap();
    //~^ parsed_string_literals
    _ = "c".parse::<char>().unwrap();
    //~^ parsed_string_literals
    _ = r#"""#.parse::<char>().unwrap();
    //~^ parsed_string_literals
    _ = "'".parse::<char>().unwrap();
    //~^ parsed_string_literals

    // Since the context provides the type to use for the result of `parse()`,
    // do not include a suffix when issuing the constant.
    let _: i64 = "-17".parse().unwrap();
    //~^ parsed_string_literals

    // Check that the original form is preserved ('🦀' == '\u{1f980}')
    _ = "\u{1f980}".parse::<char>().unwrap();
    //~^ parsed_string_literals
    _ = "🦀".parse::<char>().unwrap();
    //~^ parsed_string_literals

    // Do not lint invalid values
    _ = "-10".parse::<usize>().unwrap();

    // Ensure that trailing `+` is removed
    _ = "+10".parse::<usize>().unwrap();
    //~^ parsed_string_literals

    // Negative literals must be parenthesized when receivers of a method call
    let _: usize = "-10".parse::<isize>().unwrap().unsigned_abs();
    //~^ parsed_string_literals

    let _: c_int = "10".parse().unwrap();
    //~^ parsed_string_literals
    _ = "10".parse::<c_int>().unwrap();
    //~^ parsed_string_literals

    // Casts must be parenthesized when receivers of a method call
    type MySizedType = isize;
    let _: usize = "-10".parse::<MySizedType>().unwrap().unsigned_abs();
    //~^ parsed_string_literals

    // Casts must be parenthesized when arguments of a unary operator
    _ = -"-10".parse::<MySizedType>().unwrap();
    //~^ parsed_string_literals

    // Do not lint content or code coming from macros
    macro_rules! mac {
        (str) => {
            "10"
        };
        (parse $l:literal) => {
            $l.parse::<u32>().unwrap()
        };
    }
    _ = mac!(str).parse::<u32>().unwrap();
    _ = mac!(parse "10");
}
