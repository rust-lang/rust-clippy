//@no-rustfix
#![warn(clippy::assert_is_empty)]
#![crate_type = "lib"]
#![no_std]

// An `extern crate` item inside a module makes standard library types print
// through that module, here as `nested::heap::...`. Such a path does not resolve
// from other modules, so these suggestions are `MaybeIncorrect`. They do not
// compile.

mod nested {
    pub extern crate alloc as heap;
}

use nested::heap::collections::BTreeMap;
use nested::heap::string::String;

mod other {
    pub fn nested_extern_crate() {
        let array = [super::String::new()];
        assert!(array.is_empty());
        //~^ assert_is_empty

        let map = super::BTreeMap::<u8, u8>::new();
        assert!(map.is_empty());
        //~^ assert_is_empty
    }
}
