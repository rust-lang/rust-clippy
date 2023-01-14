//@run-rustfix

#![allow(unused)]
#![deny(clippy::as_underscore_ptr)]

use std::rc::Rc;

// intent: turn a `&u64` into a `*const u8` that points to the bytes of the u64 (⚠️does not work⚠️)
fn owo(x: &u64) -> *const u8 {
    // ⚠️ `&x` is a `&&u64`, so this turns a double pointer into a single pointer
    // ⚠️ This pointer is a dangling pointer to a local
    &x as *const _ as *const u8
}

// note: this test is the same basic idea as above, but uses a `&self` which can be even more
// misleading. In fact this is the case that was found in *real code* (that didn't work)
// that inspired this lint. Make sure that it lints with `&self`!
struct UwU;
impl UwU {
    // like above, this creates a double pointer, and then returns a dangling single pointer
    // intent: turn a `&UwU` into a `*const u8` that points to the same data
    fn as_ptr(&self) -> *const u8 {
        // ⚠️ `&self` is a `&&UwU`, so this turns a double pointer into a single pointer
        // ⚠️ This pointer is a dangling pointer to a local
        &self as *const _ as *const u8
    }
}

fn use_ptr(_: *const ()) {}

fn main() {
    let _: *const u8 = 1 as *const _;
    use_ptr(1 as *const _);

    let _: *mut u8 = 1 as *mut _;

    // Pointer-to-pointer note tests
    // If a _ resolves to a type that is itself a pointer, it's likely a mistake
    // Show a note for all of these cases

    // const ptr to ref
    let r = &&1;
    let _ = r as *const _;

    // const ptr to mut ref
    let r = &&mut 1;
    let _ = r as *const _;

    // mut ptr to ref
    let r = &mut &1;
    let _ = r as *mut _;

    // mut ptr to mut ref
    let r = &mut &mut 1;
    let _ = r as *mut _;

    // ptr to Box
    let b = Box::new(1);
    let r = &b;
    let _ = r as *const _;

    // ptr to Rc
    let rc = Rc::new(1);
    let r = &rc;
    let _ = r as *const _;
}
