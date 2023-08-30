#![warn(clippy::string_from_utf8_as_bytes)]

fn main() {
    let _ = std::str::from_utf8(&"Hello World!".as_bytes()[6..11]);
    //~^ ERROR: calling a slice of `as_bytes()` with `from_utf8` should be not necessary
    //~| NOTE: `-D clippy::string-from-utf8-as-bytes` implied by `-D warnings`
}
