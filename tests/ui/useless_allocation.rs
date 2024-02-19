#![warn(clippy::useless_allocation)]

use std::collections::HashSet;

fn main() {
    let mut s = HashSet::from(["a".to_string()]);
    s.remove(&"b".to_owned()); //~ ERROR: unneeded allocation
    s.remove(&"b".to_string()); //~ ERROR: unneeded allocation
    // Should not warn.
    s.remove("b");

    let mut s = HashSet::from([vec!["a"]]);
    s.remove(&["b"].to_vec()); //~ ERROR: unneeded allocation

    // Should not warn.
    s.remove(&["b"].to_vec().clone());
    s.remove(["a"].as_slice());
}
