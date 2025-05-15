#![warn(clippy::long_variable_names)]

fn a_function(ferris_singlehandedly_refactored_the_monolith_while_juggling_crates_and_lifetimes: &str) {
    //~^ long_variable_names
}

fn another_function(just_a_short_name: &str) {
    // should not cause a problem
}

fn main() {
    // `ferris_singlehandedly_refactored_the_monolith_while_juggling_crates_and_lifetimes` is too long
    let ferris_singlehandedly_refactored_the_monolith_while_juggling_crates_and_lifetimes = "very long indeed";
    //~^ long_variable_names
}
