#![warn(clippy::needless_path_new)]

use std::fs;
use std::path::Path;

fn takes_path(_: &Path) {}

fn takes_path_and_impl_path(_: &Path, _: impl AsRef<Path>) {}

fn main() {
    fs::write(Path::new("foo.txt"), "foo"); //~ needless_path_new

    fs::copy(
        Path::new("foo"), //~ needless_path_new
        Path::new("bar"), //~ needless_path_new
    );

    // no warning
    takes_path(Path::new("foo"));

    // the paramater that _could_ be passed directly, was
    // the parameter that could't, wasn't
    takes_path_and_impl_path(Path::new("foo"), "foo");
}
