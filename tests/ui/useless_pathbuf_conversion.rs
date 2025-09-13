#![warn(clippy::useless_pathbuf_conversion)]
use std::path::{Path, PathBuf};

fn use_path(p: &Path) {}

fn main() {
    use_path(&PathBuf::from("abc"));
    //~^ useless_pathbuf_conversion

    use_path(Path::new("abc"));
}
