#![warn(clippy::needless_path_new)]

use std::fs;
use std::path::Path;

fn main() {
    fs::write(Path::new("foo.txt"), "foo");
}
