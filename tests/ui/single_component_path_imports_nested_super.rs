// edition:2018
#![warn(clippy::single_component_path_imports)]
#![allow(unused_imports)]

use regex;
use serde;

fn main() {}

mod root_nested_use_mod {
    //use crate::regex;
    mod internal_1 {
        use crate::serde;
        mod internal_2 {
            use super::super::super::regex;
        }
    }
    #[allow(dead_code)]
    fn root_nested_use_mod() {}
}
