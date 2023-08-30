#![allow(unused_must_use)]
#![warn(clippy::writeln_empty_string)]
use std::io::Write;

fn main() {
    let mut v = Vec::new();

    // These should fail
    writeln!(v, "");
    //~^ ERROR: empty string literal in `writeln!`
    //~| NOTE: `-D clippy::writeln-empty-string` implied by `-D warnings`

    let mut suggestion = Vec::new();
    writeln!(suggestion, "");
    //~^ ERROR: empty string literal in `writeln!`

    // These should be fine
    writeln!(v);
    writeln!(v, " ");
    write!(v, "");
}
