// edition:2018
#![warn(clippy::single_component_path_imports)]
#![allow(unused_imports)]

use regex;

mod foo {
    use crate::regex::Regex;

    pub fn bar() {
        Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    }
}

fn main() {
    foo::bar()
}
