// ignore-macos

#![feature(no_core, lang_items, start)]
#![no_core]
#![allow(clippy::missing_safety_doc)]

#[link(name = "c")]
extern "C" {}

#[lang = "sized"]
pub trait Sized {}
#[lang = "copy"]
pub trait Copy {}
#[lang = "freeze"]
pub unsafe trait Freeze {}

#[lang = "start"]
fn start<T>(_main: fn() -> T, _argc: isize, _argv: *const *const u8, _sigpipe: u8) -> isize {
    0
}

fn main() {}

struct A;

impl A {
    pub fn as_ref(self) -> &'static str {
        "A"
    }
}
