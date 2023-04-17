#![allow(unused)]
#![warn(clippy::path_join_correction)]

fn main() {
  // should be linted
  let path = std::path::Path::new("/bin");
  path.join("/sh");
  println!("{}", path.display());

  //should not be linted
  let path = std::path::Path::new("/bin");
  path.join("sh");
  println!("{}", path.display());
  
}
