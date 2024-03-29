#![warn(clippy::duplicate_map_keys)]
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

fn main() {
    // true positive
    let _ = HashMap::from([(5, 1), (5, 2)]);
    let _ = HashMap::from([((), 1), ((), 2)]);
    let _ = HashMap::from([("a", 1), ("a", 2)]);

    // true negative
    let _ = HashMap::from([(5, 1), (6, 2)]);
    let _ = HashMap::from([("a", 1), ("b", 2)]);

    // false negative
    let _ = HashMap::from([(Bad(true), 1), (Bad(false), 2)]);
}

// Construction of false negative
#[derive(PartialEq)]
struct Bad(bool);

// eq is used in the lint, but hash in the HashMap
impl Eq for Bad {}

impl Hash for Bad {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // empty
    }
}
