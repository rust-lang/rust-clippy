#![warn(clippy::suggest_const_thread_local)]
use std::cell::RefCell;


fn main() {
    // lint and suggest const
    thread_local! {
        static buf1: RefCell<String> = RefCell::new(String::new());
    }

    // don't lint
    thread_local! {
        static buf2: RefCell<String> = const { RefCell::new(String::new()) };
    }

    thread_local! {
        static const_int:i32 = 1;
    }

    // lint and suggest const for all statics.
    thread_local! {
        static foo:i32 = const { 1 };
        static buf3: RefCell<String> = RefCell::new(String::new());
    }

}
