// run-rustfix
#![allow(unused)]
#![warn(clippy::join_absolute_path)]
use std::path::Path;

fn main() {
    // should be linted
    let path = Path::new("/bin");
    path.join("/sh");
    println!("{}", path.display());

    //should be linted
    let path = Path::new("C:\\Users");
    path.join("\\user");
    println!("{}", path.display());

    // should not be linted
    let path: &[&str] = &["/bin"];
    path.join("/sh");
    println!("{:?}", path);

    //should not be linted
    let path = Path::new("/bin");
    path.join("sh");
    println!("{}", path.display());
}
