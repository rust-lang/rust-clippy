#![warn(clippy::owned_cow)]

use std::borrow::Cow;
use std::ffi::{CString, OsString};
use std::path::PathBuf;

fn main() {
    let x: Cow<'static, String> = Cow::Owned(String::from("Hi!"));
    let y: Cow<'_, Vec<u8>> = Cow::Owned(vec![]);
    let z: Cow<'_, Vec<_>> = Cow::Owned(vec![2_i32]);
    let o: Cow<'_, OsString> = Cow::Owned(OsString::new());
    let c: Cow<'_, CString> = Cow::Owned(CString::new("").unwrap());
    let p: Cow<'_, PathBuf> = Cow::Owned(PathBuf::new());

    // false positive: borrowed type
    let b: Cow<'_, str> = Cow::Borrowed("Hi!");
}
