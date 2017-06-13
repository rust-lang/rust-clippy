![feature(plugin)]
![plugin(clippy)]

![deny(const_static_lifetime)]



const VAR_ONE: &'static str = "Test constant #1"; // ERROR Consider removing 'static.

const VAR_TWO: &str = "Test constant #2"; // This line should not raise a warning.




fn main() {}
