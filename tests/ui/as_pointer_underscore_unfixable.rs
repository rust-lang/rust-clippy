#![warn(clippy::as_pointer_underscore)]
#![crate_type = "lib"]
#![no_std]

fn issue_15281() {
    fn bar(_: usize) {}
    // pointer to fn item, lint should not trigger
    let _ = &bar as *const _;
    //~^ as_pointer_underscore

    let closure = &|| {};
    let _ = &closure as *const _;
    //~^ as_pointer_underscore
}
