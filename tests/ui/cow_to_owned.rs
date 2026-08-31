#![warn(clippy::cow_to_owned)]

use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::Path;

fn bytes() -> Cow<'static, [u8]> {
    Cow::Borrowed(&[])
}
fn path() -> Cow<'static, Path> {
    Cow::Borrowed(Path::new(""))
}
fn str() -> Cow<'static, str> {
    Cow::Borrowed("")
}
fn osstr() -> Cow<'static, OsStr> {
    Cow::Borrowed(OsStr::new(""))
}

fn main() {
    let _a = bytes().to_vec();
    //~^ cow_to_owned
    let _b = path().to_path_buf();
    //~^ cow_to_owned
    let _c = str().to_string();
    //~^ cow_to_owned
    let _d = osstr().to_os_string();
    //~^ cow_to_owned

    let _other = osstr().clone();
    #[allow(clippy::suspicious_to_owned)]
    let _other = osstr().to_owned();
}
