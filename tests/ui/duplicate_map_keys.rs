#![warn(clippy::duplicate_map_keys)]
use std::collections::HashMap;

fn main() {
    let example = HashMap::from([(5, 1), (5, 2)]);
}
