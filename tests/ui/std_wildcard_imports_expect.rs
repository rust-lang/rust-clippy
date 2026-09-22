//@no-rustfix

#![warn(clippy::std_wildcard_imports, clippy::wildcard_imports)]

#[expect(clippy::wildcard_imports)]
use std::rc::*;
//~^ std_wildcard_imports

fn main() {
    let _ = Rc::new(());
}
