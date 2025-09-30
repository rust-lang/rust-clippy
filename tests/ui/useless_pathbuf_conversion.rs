#![warn(clippy::useless_pathbuf_conversion)]
use std::path::{Path, PathBuf};

fn use_path(p: &Path) {}

fn takes_ref_pathbuf(_: &PathBuf) {}

fn main() {
    use_path(&PathBuf::from("abc"));
    //~^ useless_pathbuf_conversion

    use_path(Path::new("abc"));

    takes_ref_pathbuf(&PathBuf::from("path"));
}
