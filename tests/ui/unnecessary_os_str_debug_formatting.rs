#![feature(os_str_display)]
#![warn(clippy::unnecessary_debug_formatting)]

use std::ffi::{OsStr, OsString};

fn main() {
    let os_str = OsStr::new("abc");
    let os_string = os_str.to_os_string();

    // negative tests
    println!("{}", os_str.display());
    println!("{}", os_string.display());

    // positive tests
    println!("{:?}", os_str);
    println!("{:?}", os_string);

    let _: String = format!("{:?}", os_str);
    let _: String = format!("{:?}", os_string);
}
