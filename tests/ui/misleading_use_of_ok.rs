#![warn(clippy::misleading_use_of_ok)]
#![allow(dead_code)]

fn bad_style(x: &str) {
    x.parse::<u32>().ok();
}

fn good_style(x: &str) -> Option<u32> {
    x.parse::<u32>().ok()
}

#[rustfmt::skip]
fn strange_parse(x: &str) {
    x   .   parse::<i32>()   .   ok   ();
}

fn main() {}
