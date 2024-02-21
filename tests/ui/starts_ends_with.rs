#![allow(clippy::needless_if, dead_code, unused_must_use)]

fn main() {}

#[allow(clippy::unnecessary_operation)]
fn starts_with() {
    "".chars().next() == Some(' '); //~ chars_next_cmp
    Some(' ') != "".chars().next(); //~ chars_next_cmp

    // Ensure that suggestion is escaped correctly
    "".chars().next() == Some('\n'); //~ chars_next_cmp
    Some('\n') != "".chars().next(); //~ chars_next_cmp
}

fn chars_cmp_with_unwrap() {
    let s = String::from("foo");
    //~v chars_next_cmp
    if s.chars().next().unwrap() == 'f' {
        // s.starts_with('f')
        // Nothing here
    }
    //~v chars_last_cmp
    if s.chars().next_back().unwrap() == 'o' {
        // s.ends_with('o')
        // Nothing here
    }
    //~v chars_last_cmp
    if s.chars().last().unwrap() == 'o' {
        // s.ends_with('o')
        // Nothing here
    }
    //~v chars_next_cmp
    if s.chars().next().unwrap() != 'f' {
        // !s.starts_with('f')
        // Nothing here
    }
    //~v chars_last_cmp
    if s.chars().next_back().unwrap() != 'o' {
        // !s.ends_with('o')
        // Nothing here
    }
    //~v chars_last_cmp
    if s.chars().last().unwrap() != '\n' {
        // !s.ends_with('o')
        // Nothing here
    }
}

#[allow(clippy::unnecessary_operation)]
fn ends_with() {
    "".chars().last() == Some(' '); //~ chars_last_cmp
    Some(' ') != "".chars().last(); //~ chars_last_cmp
    "".chars().next_back() == Some(' '); //~ chars_last_cmp
    Some(' ') != "".chars().next_back(); //~ chars_last_cmp

    // Ensure that suggestion is escaped correctly
    "".chars().last() == Some('\n'); //~ chars_last_cmp
    Some('\n') != "".chars().last(); //~ chars_last_cmp
}
