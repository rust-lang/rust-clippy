#![allow(clippy::no_effect, unused)]
#![warn(clippy::needless_raw_string_hashes)]

fn main() {
    r#"\aaa"#; //~ needless_raw_string_hashes
    r##"Hello "world"!"##; //~ needless_raw_string_hashes
    r######" "### "## "# "######; //~ needless_raw_string_hashes
    r######" "aa" "# "## "######; //~ needless_raw_string_hashes
    br#"\aaa"#; //~ needless_raw_string_hashes
    br##"Hello "world"!"##; //~ needless_raw_string_hashes
    br######" "### "## "# "######; //~ needless_raw_string_hashes
    br######" "aa" "# "## "######; //~ needless_raw_string_hashes
    cr#"\aaa"#; //~ needless_raw_string_hashes
    cr##"Hello "world"!"##; //~ needless_raw_string_hashes
    cr######" "### "## "# "######; //~ needless_raw_string_hashes
    cr######" "aa" "# "## "######; //~ needless_raw_string_hashes

    //~v needless_raw_string_hashes
    r#"
        \a
        multiline
        string
    "#;

    r###"rust"###; //~ needless_raw_string_hashes
    r#"hello world"#; //~ needless_raw_string_hashes
}
