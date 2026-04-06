//@no-rustfix
#![warn(clippy::vec_to_rc_slice)]

use std::rc::Rc;
use std::sync::Arc;

fn accept_arc_slice(_: Arc<[u8]>) {}
fn accept_rc_slice(_: Rc<[u8]>) {}

fn main() {
    // Type annotation constrains the result type; fix changes it
    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Arc<[u8]> = v.into();
    //~^ vec_to_rc_slice

    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Arc<[u8]> = Arc::from(v);
    //~^ vec_to_rc_slice

    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Arc<[u8]> = From::from(v);
    //~^ vec_to_rc_slice

    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Rc<[u8]> = v.into();
    //~^ vec_to_rc_slice

    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Rc<[u8]> = Rc::from(v);
    //~^ vec_to_rc_slice

    let v: Vec<u8> = vec![1, 2, 3];
    let _a: Rc<[u8]> = From::from(v);
    //~^ vec_to_rc_slice

    // Function signature constrains the result type; fix requires downstream changes
    let v: Vec<u8> = vec![1, 2, 3];
    accept_arc_slice(v.into());
    //~^ vec_to_rc_slice

    let v: Vec<u8> = vec![1, 2, 3];
    accept_rc_slice(Rc::from(v));
    //~^ vec_to_rc_slice
}
