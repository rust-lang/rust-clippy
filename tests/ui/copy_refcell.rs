#![warn(clippy::copy_refcell)]

use std::cell::RefCell;

struct MyStruct {
    field: RefCell<u8>,
}

fn main() {
    let local = RefCell::new(0_u8);
    let large_local = RefCell::new([0_u8; 1024]);
}
