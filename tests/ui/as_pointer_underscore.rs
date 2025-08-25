#![warn(clippy::as_pointer_underscore)]
#![crate_type = "lib"]
#![no_std]

struct S;

fn f(s: &S) -> usize {
    &s as *const _ as usize
    //~^ as_pointer_underscore
}

fn g(s: &mut S) -> usize {
    s as *mut _ as usize
    //~^ as_pointer_underscore
}

fn issue_15281() {
    fn bar(_: usize) {}
    // pointer to fn item, lint should not trigger
    let _ = &bar as *const _;
    let _ = &(bar as fn(usize)) as *const _;
    //~^ as_pointer_underscore
}
