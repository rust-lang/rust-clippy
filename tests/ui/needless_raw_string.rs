#![allow(clippy::needless_raw_string_hashes, clippy::no_effect, unused)]
#![warn(clippy::needless_raw_strings)]

fn main() {
    r#"aaa"#;
    r#""aaa""#;
    r#"\s"#;
    br#"aaa"#;
    br#""aaa""#;
    br#"\s"#;
    cr#"aaa"#;
    cr#""aaa""#;
    cr#"\s"#;

    r#"
        a
        multiline
        string
    "#;

    r"no hashes";
    br"no hashes";
    cr"no hashes";
}
