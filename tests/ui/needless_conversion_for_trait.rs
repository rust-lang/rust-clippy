#![allow(unused_imports, unused_parens, clippy::let_unit_value, clippy::manual_async_fn)]
#![feature(str_as_str)]

use std::borrow::{Borrow, BorrowMut};
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let mut s = String::new();
    let mut vec = Vec::<u8>::new();
    let mut path_buf = PathBuf::from("x");
    let osstr = OsStr::new("");
    let osstring = OsString::new();
    let path = Path::new("x");
    let mut readable = Box::new(&[] as &[u8]);

    // inherent functions

    let _ = std::fs::write("x", (Box::new([]) as Box<[u8]>).into_vec()); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", (&[] as &[u8]).to_vec()); //~ needless_conversion_for_trait

    let _ = is_empty(s.clone().into_boxed_str().into_boxed_bytes()); //~ needless_conversion_for_trait
    let _ = is_empty(s.clone().into_boxed_str().into_string()); //~ needless_conversion_for_trait

    let _ = std::fs::write("x", s.as_bytes()); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", s.as_mut_str()); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", s.as_str()); //~ needless_conversion_for_trait
    let _ = is_empty(s.clone().into_boxed_str()); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", s.clone().into_bytes()); //~ needless_conversion_for_trait

    let _ = std::fs::write("x", vec.as_mut_slice()); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", vec.as_slice()); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", vec.into_boxed_slice()); //~ needless_conversion_for_trait

    let _ = Command::new("ls").args(["-a", "-l"].iter()); //~ needless_conversion_for_trait
    let _ = Command::new("ls").args(["-a", "-l"].iter_mut()); //~ needless_conversion_for_trait

    let _ = std::fs::write("x", "".as_bytes()); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", "".as_str()); //~ needless_conversion_for_trait

    let _ = os_str_or_bytes(osstr.as_encoded_bytes()); //~ needless_conversion_for_trait
    let _ = is_empty_os(osstring.clone().into_boxed_os_str().into_os_string()); //~ needless_conversion_for_trait
    let _ = std::fs::write(OsStr::new("x"), ""); //~ needless_conversion_for_trait
    let _ = std::fs::write(osstr.to_os_string(), ""); //~ needless_conversion_for_trait

    let _ = std::fs::write(osstring.as_os_str(), ""); //~ needless_conversion_for_trait
    let _ = is_empty_os(osstring.clone().into_boxed_os_str()); //~ needless_conversion_for_trait
    let _ = os_string_or_bytes(osstring.clone().into_encoded_bytes()); //~ needless_conversion_for_trait

    let _ = std::fs::write(PathBuf::from("x").as_mut_os_str(), ""); //~ needless_conversion_for_trait
    let _ = std::fs::write(path.as_os_str(), ""); //~ needless_conversion_for_trait
    let _ = std::fs::write(PathBuf::from("x").into_boxed_path().into_path_buf(), ""); //~ needless_conversion_for_trait
    let _ = Command::new("ls").args(path.iter()); //~ needless_conversion_for_trait
    let _ = std::fs::write(Path::new("x"), ""); //~ needless_conversion_for_trait
    let _ = std::fs::write(path.to_path_buf(), ""); //~ needless_conversion_for_trait

    let _ = std::fs::write(path_buf.as_mut_os_string(), ""); //~ needless_conversion_for_trait
    let _ = std::fs::write(path_buf.as_path(), ""); //~ needless_conversion_for_trait
    let _ = std::fs::write(path_buf.clone().into_os_string(), ""); //~ needless_conversion_for_trait

    // trait methods

    let _ = std::fs::write("x", "".to_owned()); //~ needless_conversion_for_trait

    let _ = std::fs::write("x", "".to_string()); //~ needless_conversion_for_trait

    let _ = std::fs::write("x", <_ as Borrow<str>>::borrow(&s)); //~ needless_conversion_for_trait

    read(<_ as BorrowMut<&[u8]>>::borrow_mut(&mut readable)); //~ needless_conversion_for_trait
    read(<_ as BorrowMut<Box<_>>>::borrow_mut(&mut readable)); //~ needless_conversion_for_trait

    read(readable.as_mut()); //~ needless_conversion_for_trait

    let _ = std::fs::write("x", <_ as AsRef<[u8]>>::as_ref("")); //~ needless_conversion_for_trait
    let _ = std::fs::write("x", <_ as AsRef<str>>::as_ref("")); //~ needless_conversion_for_trait

    let _ = std::fs::write("x", s.deref()); //~ needless_conversion_for_trait

    read(readable.deref_mut()); //~ needless_conversion_for_trait
}

#[must_use]
fn is_empty<T: From<Box<str>> + PartialEq>(x: T) -> bool {
    x == T::from(String::new().into_boxed_str())
}

#[must_use]
fn is_empty_os<T: From<Box<OsStr>> + PartialEq>(x: T) -> bool {
    x == T::from(OsString::new().into_boxed_os_str())
}

// This is a hack, but I can't readily think of a trait that both `&OsStr` and `&[u8]` implement.
// Reference: https://github.com/rust-lang/rust/issues/111544
trait OsStrOrBytes {}
impl OsStrOrBytes for &OsStr {}
impl OsStrOrBytes for &[u8] {}
fn os_str_or_bytes(_: impl OsStrOrBytes) {}

// Similar hack for `OsString` and `Vec<u8>`.
// Reference: https://github.com/rust-lang/rust/issues/111544
trait OsStringOrBytes {}
impl OsStringOrBytes for OsString {}
impl OsStringOrBytes for Vec<u8> {}
fn os_string_or_bytes(_: impl OsStringOrBytes) {}

fn read(_: impl Read) {}
