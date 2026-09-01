#![warn(clippy::cow_to_owned)]
#![expect(clippy::suspicious_to_owned)]

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

fn cow_ref() -> &'static Cow<'static, [u8]> {
    &Cow::Borrowed(&[])
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
    let _other = osstr().to_owned();
    let _other = cow_ref().to_vec();
    non_template_values();
}

// Perhaps in the future, single reference or last reference may be possible
fn non_template_values() {
    let single = bytes();
    let multi = bytes();

    let _a = single.to_vec();
    let _b = multi.to_vec();
    let _c = multi.to_vec();
}
