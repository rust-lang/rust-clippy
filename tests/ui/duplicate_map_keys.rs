#![warn(clippy::duplicate_map_keys)]
#![allow(clippy::diverging_sub_expression)]
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

fn main() -> Result<(), ()> {
    let _ = HashMap::from([(5, 1), (5, 2)]); // expect lint
    let _ = HashMap::from([((), 1), ((), 2)]); // expect lint
    let _ = HashMap::from([("a", 1), ("a", 2)]); // expect lint

    let _ = HashMap::from([(return Ok(()), 1), (return Err(()), 2)]); // expect no lint 
    // is this what you meant? I'm not sure how to put return into there

    let _ = HashMap::from([(5, 1), (6, 2)]); // expect no lint
    let _ = HashMap::from([("a", 1), ("b", 2)]); // expect no lint

    let _ = HashMap::from([(Bad(true), 1), (Bad(false), 2)]); // expect no lint (false negative)

    Ok(())
}

// Construction of false negative
struct Bad(bool);

// eq is used in the lint, but hash in the HashMap
impl Eq for Bad {}

impl PartialEq for Bad {
    fn eq(&self, other: &Self) -> bool {
        true
    }
}

impl Hash for Bad {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // empty
    }
}
