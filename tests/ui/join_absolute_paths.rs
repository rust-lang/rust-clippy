#![allow(unused)]
#![warn(clippy::join_absolute_paths)]
use std::path::Path;

fn main() {
    // should be linted
    let path = Path::new("/bin");
    path.join("/sh");

    //should be linted
    let path = Path::new("C:\\Users");
    path.join("\\user");

    // should not be linted
    let path: &[&str] = &["/bin"];
    path.join("/sh");

    //should not be linted
    let path = Path::new("/bin");
    path.join("sh");
}
