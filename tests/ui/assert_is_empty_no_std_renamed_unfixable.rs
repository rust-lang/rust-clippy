//@no-rustfix
#![warn(clippy::assert_is_empty)]
#![crate_type = "lib"]
#![no_std]

// Standard library types print through the `extern crate` item that names their
// crate, here as `memory::string::String`. Only `std`, `core`, and `alloc` roots are
// trusted, so this suggestion is `MaybeIncorrect`. It does not compile, because the
// local `memory` module shadows the renamed crate.

extern crate alloc as memory;

use memory::string::String;

mod shadowed {
    mod memory {}

    pub fn renamed_crate() {
        let array = [super::String::new()];
        assert!(array.is_empty());
        //~^ assert_is_empty
    }
}
