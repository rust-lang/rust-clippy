#![feature(rustc_private)]

extern crate rustc_driver;

fn main() {
    print!("{}", clippy_config::Conf::schema_json());
}
