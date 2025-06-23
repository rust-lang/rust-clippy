#![warn(clippy::needless_path_new)]

use std::fs;
use std::path::Path;

fn foo() -> Option<&'static Path> {
    // Some(...) is `ExprKind::Call`, but we don't consider it
    Some(Path::new("foo.txt"))
}

fn main() {
    let _: Option<&Path> = Some(Path::new("foo"));
    fn foo() -> Option<impl AsRef<Path>> {
        Some(Path::new("bar.txt")) //~ needless_path_new
    }
}
