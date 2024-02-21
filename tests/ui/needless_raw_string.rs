#![allow(clippy::needless_raw_string_hashes, clippy::no_effect, unused)]
#![warn(clippy::needless_raw_strings)]

fn main() {
    r#"aaa"#; //~ needless_raw_strings
    r#""aaa""#;
    r#"\s"#;
    br#"aaa"#; //~ needless_raw_strings
    br#""aaa""#;
    br#"\s"#;
    cr#"aaa"#; //~ needless_raw_strings
    cr#""aaa""#;
    cr#"\s"#;

    //~v needless_raw_strings
    r#"
        a
        multiline
        string
    "#;

    r"no hashes"; //~ needless_raw_strings
    br"no hashes"; //~ needless_raw_strings
    cr"no hashes"; //~ needless_raw_strings
}
