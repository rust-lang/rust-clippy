//@no-rustfix
#![warn(clippy::vec_to_rc_slice)]

use std::rc::Rc;
use std::sync::Arc;

fn accept_arc_slice(_: Arc<[u8]>) {}
fn accept_rc_slice(_: Rc<[u8]>) {}

fn main() {
    // Should lint: result passed to a function expecting Arc<[T]> (fix requires downstream changes)
    let v: Vec<u8> = vec![1, 2, 3];
    accept_arc_slice(v.into());
    //~^ vec_to_rc_slice

    // Should lint: result passed to a function expecting Rc<[T]> (fix requires downstream changes)
    let v: Vec<u8> = vec![1, 2, 3];
    accept_rc_slice(Rc::from(v));
    //~^ vec_to_rc_slice
}
