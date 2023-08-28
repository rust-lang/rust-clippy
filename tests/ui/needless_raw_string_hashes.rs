#![allow(clippy::no_effect, unused)]
#![warn(clippy::needless_raw_string_hashes)]
#![feature(c_str_literals)]

fn main() {
    r#"aaa"#;
    r##"Hello "world"!"##;
    //~^ ERROR: unnecessary hashes around raw string literal
    //~| NOTE: `-D clippy::needless-raw-string-hashes` implied by `-D warnings`
    r######" "### "## "# "######;
    //~^ ERROR: unnecessary hashes around raw string literal
    r######" "aa" "# "## "######;
    //~^ ERROR: unnecessary hashes around raw string literal
    br#"aaa"#;
    br##"Hello "world"!"##;
    //~^ ERROR: unnecessary hashes around raw string literal
    br######" "### "## "# "######;
    //~^ ERROR: unnecessary hashes around raw string literal
    br######" "aa" "# "## "######;
    //~^ ERROR: unnecessary hashes around raw string literal
    // currently disabled: https://github.com/rust-lang/rust/issues/113333
    // cr#"aaa"#;
    // cr##"Hello "world"!"##;
    // cr######" "### "## "# "######;
    // cr######" "aa" "# "## "######;
}
