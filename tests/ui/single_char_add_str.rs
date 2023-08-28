#![warn(clippy::single_char_add_str)]
#![allow(clippy::needless_raw_strings, clippy::needless_raw_string_hashes)]

macro_rules! get_string {
    () => {
        String::from("Hello world!")
    };
}

fn main() {
    // `push_str` tests

    let mut string = String::new();
    string.push_str("R");
    //~^ ERROR: calling `push_str()` using a single-character string literal
    //~| NOTE: `-D clippy::single-char-add-str` implied by `-D warnings`
    string.push_str("'");
    //~^ ERROR: calling `push_str()` using a single-character string literal

    string.push('u');
    string.push_str("st");
    string.push_str("");
    string.push_str("\x52");
    //~^ ERROR: calling `push_str()` using a single-character string literal
    string.push_str("\u{0052}");
    //~^ ERROR: calling `push_str()` using a single-character string literal
    string.push_str(r##"a"##);
    //~^ ERROR: calling `push_str()` using a single-character string literal

    get_string!().push_str("ö");
    //~^ ERROR: calling `push_str()` using a single-character string literal

    // `insert_str` tests

    let mut string = String::new();
    string.insert_str(0, "R");
    //~^ ERROR: calling `insert_str()` using a single-character string literal
    string.insert_str(1, "'");
    //~^ ERROR: calling `insert_str()` using a single-character string literal

    string.insert(0, 'u');
    string.insert_str(2, "st");
    string.insert_str(0, "");
    string.insert_str(0, "\x52");
    //~^ ERROR: calling `insert_str()` using a single-character string literal
    string.insert_str(0, "\u{0052}");
    //~^ ERROR: calling `insert_str()` using a single-character string literal
    let x: usize = 2;
    string.insert_str(x, r##"a"##);
    //~^ ERROR: calling `insert_str()` using a single-character string literal
    const Y: usize = 1;
    string.insert_str(Y, r##"a"##);
    //~^ ERROR: calling `insert_str()` using a single-character string literal
    string.insert_str(Y, r##"""##);
    //~^ ERROR: calling `insert_str()` using a single-character string literal
    string.insert_str(Y, r##"'"##);
    //~^ ERROR: calling `insert_str()` using a single-character string literal

    get_string!().insert_str(1, "?");
    //~^ ERROR: calling `insert_str()` using a single-character string literal
}
