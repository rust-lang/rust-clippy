#![allow(clippy::needless_raw_strings, clippy::needless_raw_string_hashes, unused_must_use)]

use std::collections::HashSet;

fn main() {
    let x = "foo";
    x.split("x");
    //~^ ERROR: single-character string constant used as pattern
    //~| NOTE: `-D clippy::single-char-pattern` implied by `-D warnings`
    x.split("xx");
    x.split('x');

    let y = "x";
    x.split(y);
    x.split("ß");
    //~^ ERROR: single-character string constant used as pattern
    x.split("ℝ");
    //~^ ERROR: single-character string constant used as pattern
    x.split("💣");
    //~^ ERROR: single-character string constant used as pattern
    // Can't use this lint for unicode code points which don't fit in a char
    x.split("❤️");
    x.split_inclusive("x");
    //~^ ERROR: single-character string constant used as pattern
    x.contains("x");
    //~^ ERROR: single-character string constant used as pattern
    x.starts_with("x");
    //~^ ERROR: single-character string constant used as pattern
    x.ends_with("x");
    //~^ ERROR: single-character string constant used as pattern
    x.find("x");
    //~^ ERROR: single-character string constant used as pattern
    x.rfind("x");
    //~^ ERROR: single-character string constant used as pattern
    x.rsplit("x");
    //~^ ERROR: single-character string constant used as pattern
    x.split_terminator("x");
    //~^ ERROR: single-character string constant used as pattern
    x.rsplit_terminator("x");
    //~^ ERROR: single-character string constant used as pattern
    x.splitn(2, "x");
    //~^ ERROR: single-character string constant used as pattern
    x.rsplitn(2, "x");
    //~^ ERROR: single-character string constant used as pattern
    x.split_once("x");
    //~^ ERROR: single-character string constant used as pattern
    x.rsplit_once("x");
    //~^ ERROR: single-character string constant used as pattern
    x.matches("x");
    //~^ ERROR: single-character string constant used as pattern
    x.rmatches("x");
    //~^ ERROR: single-character string constant used as pattern
    x.match_indices("x");
    //~^ ERROR: single-character string constant used as pattern
    x.rmatch_indices("x");
    //~^ ERROR: single-character string constant used as pattern
    x.trim_start_matches("x");
    //~^ ERROR: single-character string constant used as pattern
    x.trim_end_matches("x");
    //~^ ERROR: single-character string constant used as pattern
    x.strip_prefix("x");
    //~^ ERROR: single-character string constant used as pattern
    x.strip_suffix("x");
    //~^ ERROR: single-character string constant used as pattern
    x.replace("x", "y");
    //~^ ERROR: single-character string constant used as pattern
    x.replacen("x", "y", 3);
    //~^ ERROR: single-character string constant used as pattern
    // Make sure we escape characters correctly.
    x.split("\n");
    //~^ ERROR: single-character string constant used as pattern
    x.split("'");
    //~^ ERROR: single-character string constant used as pattern
    x.split("\'");
    //~^ ERROR: single-character string constant used as pattern

    let h = HashSet::<String>::new();
    h.contains("X"); // should not warn

    x.replace(';', ",").split(","); // issue #2978
    //~^ ERROR: single-character string constant used as pattern
    x.starts_with("\x03"); // issue #2996
    //~^ ERROR: single-character string constant used as pattern

    // Issue #3204
    const S: &str = "#";
    x.find(S);

    // Raw string
    x.split(r"a");
    //~^ ERROR: single-character string constant used as pattern
    x.split(r#"a"#);
    //~^ ERROR: single-character string constant used as pattern
    x.split(r###"a"###);
    //~^ ERROR: single-character string constant used as pattern
    x.split(r###"'"###);
    //~^ ERROR: single-character string constant used as pattern
    x.split(r###"#"###);
    //~^ ERROR: single-character string constant used as pattern
    // Must escape backslash in raw strings when converting to char #8060
    x.split(r#"\"#);
    //~^ ERROR: single-character string constant used as pattern
    x.split(r"\");
    //~^ ERROR: single-character string constant used as pattern
}
