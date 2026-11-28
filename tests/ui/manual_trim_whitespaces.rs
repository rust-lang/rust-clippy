#![warn(clippy::manual_trim_whitespaces)]

fn main() {
    let some_str = " foo ";
    // all method and closure combinations
    let _ = some_str.trim_matches(|c: char| c.is_whitespace());
    //~^ manual_trim_whitespaces

    let _ = some_str.trim_matches(|c: char| c.is_ascii_whitespace());
    //~^ manual_trim_whitespaces

    let _ = some_str.trim_start_matches(|c: char| c.is_whitespace());
    //~^ manual_trim_whitespaces

    let _ = some_str.trim_start_matches(|c: char| c.is_ascii_whitespace());
    //~^ manual_trim_whitespaces

    let _ = some_str.trim_end_matches(|c: char| c.is_whitespace());
    //~^ manual_trim_whitespaces

    let _ = some_str.trim_end_matches(|c: char| c.is_ascii_whitespace());
    //~^ manual_trim_whitespaces

    // works with literals
    let _ = "a literal string".trim_start_matches(|c: char| c.is_ascii_whitespace());
    //~^ manual_trim_whitespaces

    // works within method chains
    //~v manual_trim_whitespaces
    let _ = "a literal string"
        .repeat(2)
        .trim_matches(|c: char| c.is_ascii_whitespace());

    // works inside functions
    let _ = "boo".replace("bo", "a literal string".trim_matches(|c: char| c.is_ascii_whitespace()));
    //~^ manual_trim_whitespaces

    // no trigger on wrong method call
    let _ = "qwerty".matches(|c: char| c.is_ascii_whitespace());

    // no trigger on wrong method call
    let _ = "cool_string".trim_matches(|c: char| c.is_ascii_control());

    // no trigger on different reciever call in closure
    let outer_char = 'p';
    let _ = "some_string".trim_matches(|c: char| outer_char.is_ascii_whitespace());

    // no trigger on non-str reciever
    let _ = Dummy {}.trim_matches(|c: char| c.is_ascii_whitespace());
}

#[clippy::msrv = "1.79.0"]
fn before_ascii_msrv() {
    // no lint
    let _ = "a literal string".trim_start_matches(|c: char| c.is_ascii_whitespace());
}

struct Dummy {}
impl Dummy {
    pub fn trim_matches<F>(&self, pat: F) -> &str
    where
        F: FnOnce(char) -> bool,
    {
        "dummy"
    }
}
