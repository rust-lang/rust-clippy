//@no-rustfix
#![warn(clippy::refcell_cell)]

use std::cell::RefCell;

fn main() {
    let _ = RefCell::new([0_u32; 16]); //~ refcell_cell
    let _ = RefCell::new([0_u32; 17]);
}
