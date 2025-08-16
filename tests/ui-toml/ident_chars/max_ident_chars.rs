#![warn(clippy::max_ident_chars)]

fn a_function(ferris_singlehandedly_refactored_the_monolith_while_juggling_crates_and_lifetimes: &str) {
    //~^ max_ident_chars
}

fn another_function(just_a_short_name: &str) {
    // should not cause a problem
}

fn main() {
    // `ferris_singlehandedly_refactored_the_monolith_while_juggling_crates_and_lifetimes` is too long
    let ferris_singlehandedly_refactored_the_monolith_while_juggling_crates_and_lifetimes = "very long indeed";
    //~^ max_ident_chars
}
