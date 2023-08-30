#![allow(unused)]
#![warn(clippy::manual_main_separator_str)]

use std::path::MAIN_SEPARATOR;

fn len(s: &str) -> usize {
    s.len()
}

struct U<'a> {
    f: &'a str,
    g: &'a String,
}

struct V<T> {
    f: T,
}

fn main() {
    // Should lint
    let _: &str = &MAIN_SEPARATOR.to_string();
    //~^ ERROR: taking a reference on `std::path::MAIN_SEPARATOR` conversion to `String`
    //~| NOTE: `-D clippy::manual-main-separator-str` implied by `-D warnings`
    let _ = len(&MAIN_SEPARATOR.to_string());
    //~^ ERROR: taking a reference on `std::path::MAIN_SEPARATOR` conversion to `String`
    let _: Vec<u16> = MAIN_SEPARATOR.to_string().encode_utf16().collect();
    //~^ ERROR: taking a reference on `std::path::MAIN_SEPARATOR` conversion to `String`

    // Should lint for field `f` only
    let _ = U {
        f: &MAIN_SEPARATOR.to_string(),
        //~^ ERROR: taking a reference on `std::path::MAIN_SEPARATOR` conversion to `Strin
        g: &MAIN_SEPARATOR.to_string(),
    };

    // Should not lint
    let _: &String = &MAIN_SEPARATOR.to_string();
    let _ = &MAIN_SEPARATOR.to_string();
    let _ = V {
        f: &MAIN_SEPARATOR.to_string(),
    };
}
