//@error-in-other-file: `std::nonexistent::FakeType` does not refer to a reachable type
//@error-in-other-file: `std::nonexistent::FakeTrait` does not refer to a reachable trait
//@error-in-other-file: expected a type, found a function: `std::mem::swap`
//@error-in-other-file: expected a trait, found a struct: `std::string::String`

#![warn(clippy::disallowed_trait_usage)]

fn main() {
    // None of these should trigger since all config entries are invalid
    println!("{:?}", 42_i32);
}
