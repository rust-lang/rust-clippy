#![allow(unused_imports, clippy::disallowed_name)]
#![warn(clippy::explicit_write)]

fn main() {
    use std::io::Write;
    let bar = "bar";
    writeln!(std::io::stderr(), "foo {}", bar).unwrap();
}
