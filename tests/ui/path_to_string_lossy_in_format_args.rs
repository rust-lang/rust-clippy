#![warn(clippy::to_string_lossy_in_format_args)]

use std::ops::Deref;
use std::path::Path;

macro_rules! path_to_string {
    ($path:expr) => {
        $path.to_string_lossy()
    };
}

struct DerefPath<'a> {
    path: &'a Path,
}

impl Deref for DerefPath<'_> {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        self.path
    }
}

fn main() {
    let path = Path::new("/a/b/c");
    let path_buf = path.to_path_buf();

    // negative tests
    println!("{}", path.display());
    println!("{}", path_buf.display());

    println!("{}", path_to_string!(path));

    // positive tests
    println!("{}", path.to_string_lossy()); //~ to_string_lossy_in_format_args
    println!("{}", path_buf.to_string_lossy()); //~ to_string_lossy_in_format_args

    let _: String = format!("{}", path.to_string_lossy()); //~ to_string_lossy_in_format_args
    let _: String = format!("{}", path_buf.to_string_lossy()); //~ to_string_lossy_in_format_args

    let deref_path = DerefPath { path };
    println!("{}", deref_path.to_string_lossy()); //~ to_string_lossy_in_format_args
}
