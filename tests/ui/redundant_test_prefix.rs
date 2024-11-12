#![allow(dead_code)]
#![warn(clippy::redundant_test_prefix)]

fn main() {
    // test code goes here
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main() {
        main();
    }
}

mod tests_no_annotations {
    use super::*;

    #[test]
    fn test_main() {
        main();
    }
}
