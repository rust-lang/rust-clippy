#![warn(clippy::large_include_file)]

// Good
const GOOD_INCLUDE_BYTES: &[u8] = include_bytes!("small.txt");
const GOOD_INCLUDE_STR: &str = include_str!("small.txt");

#[allow(clippy::large_include_file)]
const ALLOWED_TOO_BIG_INCLUDE_BYTES: &[u8] = include_bytes!("too_big.txt");
#[allow(clippy::large_include_file)]
const ALLOWED_TOO_BIG_INCLUDE_STR: &str = include_str!("too_big.txt");

// Bad
const TOO_BIG_INCLUDE_BYTES: &[u8] = include_bytes!("too_big.txt");
const TOO_BIG_INCLUDE_STR: &str = include_str!("too_big.txt");

fn main() {}
