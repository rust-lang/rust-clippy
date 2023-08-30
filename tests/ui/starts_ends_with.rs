#![allow(clippy::needless_if, dead_code, unused_must_use)]

fn main() {}

#[allow(clippy::unnecessary_operation)]
fn starts_with() {
    "".chars().next() == Some(' ');
    //~^ ERROR: you should use the `starts_with` method
    //~| NOTE: `-D clippy::chars-next-cmp` implied by `-D warnings`
    Some(' ') != "".chars().next();
    //~^ ERROR: you should use the `starts_with` method

    // Ensure that suggestion is escaped correctly
    "".chars().next() == Some('\n');
    //~^ ERROR: you should use the `starts_with` method
    Some('\n') != "".chars().next();
    //~^ ERROR: you should use the `starts_with` method
}

fn chars_cmp_with_unwrap() {
    let s = String::from("foo");
    if s.chars().next().unwrap() == 'f' {
    //~^ ERROR: you should use the `starts_with` method
        // s.starts_with('f')
        // Nothing here
    }
    if s.chars().next_back().unwrap() == 'o' {
    //~^ ERROR: you should use the `ends_with` method
    //~| NOTE: `-D clippy::chars-last-cmp` implied by `-D warnings`
        // s.ends_with('o')
        // Nothing here
    }
    if s.chars().last().unwrap() == 'o' {
    //~^ ERROR: you should use the `ends_with` method
        // s.ends_with('o')
        // Nothing here
    }
    if s.chars().next().unwrap() != 'f' {
    //~^ ERROR: you should use the `starts_with` method
        // !s.starts_with('f')
        // Nothing here
    }
    if s.chars().next_back().unwrap() != 'o' {
    //~^ ERROR: you should use the `ends_with` method
        // !s.ends_with('o')
        // Nothing here
    }
    if s.chars().last().unwrap() != '\n' {
    //~^ ERROR: you should use the `ends_with` method
        // !s.ends_with('o')
        // Nothing here
    }
}

#[allow(clippy::unnecessary_operation)]
fn ends_with() {
    "".chars().last() == Some(' ');
    //~^ ERROR: you should use the `ends_with` method
    Some(' ') != "".chars().last();
    //~^ ERROR: you should use the `ends_with` method
    "".chars().next_back() == Some(' ');
    //~^ ERROR: you should use the `ends_with` method
    Some(' ') != "".chars().next_back();
    //~^ ERROR: you should use the `ends_with` method

    // Ensure that suggestion is escaped correctly
    "".chars().last() == Some('\n');
    //~^ ERROR: you should use the `ends_with` method
    Some('\n') != "".chars().last();
    //~^ ERROR: you should use the `ends_with` method
}
