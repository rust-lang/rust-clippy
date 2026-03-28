//@require-annotations-for-level: WARN
#![allow(unused)]
#![warn(clippy::tests_outside_test_module)]

fn main() {
    // test code goes here
}

// Should lint
#[test]
fn my_test() {}
//~^ ERROR: this function marked with #[test] is outside a #[cfg(test)] module
//~| NOTE: move it to a testing module marked with #[cfg(test)]

#[cfg(test)]
mod tests {
    // Should not lint
    #[test]
    fn my_test() {}
}

#[allow(clippy::non_minimal_cfg)]
#[cfg(all(test))]
mod tests_all {
    // Should not lint: `all(test)` implies `test`
    #[test]
    fn my_test() {}
}

#[cfg(all(test, not(target_pointer_width = "16")))]
mod tests_all_compound {
    // Should not lint: `all(test, ...)` implies `test`
    #[test]
    fn my_test() {}
}
