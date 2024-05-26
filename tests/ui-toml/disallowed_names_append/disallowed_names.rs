#![warn(clippy::disallowed_names)]
#![allow(clippy::let_arr_const)]

fn main() {
    // `foo` is part of the default configuration
    let foo = "bar";
    // `ducks` was unrightfully disallowed
    let ducks = ["quack", "quack"];
    // `fox` is okay
    let fox = ["what", "does", "the", "fox", "say", "?"];
}
