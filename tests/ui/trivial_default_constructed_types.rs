//@run-rustfix
//@aux-build:proc_macros.rs:proc-macro
#![allow(clippy::default_constructed_unit_structs, clippy::no_effect, unused)]
#![warn(clippy::trivial_default_constructed_types)]
#![feature(generic_arg_infer)]

#[macro_use]
extern crate proc_macros;

use std::sync::atomic::{AtomicBool, AtomicI8, AtomicPtr, AtomicU8};

#[derive(Default)]
struct OtherType;

impl Default for &OtherType {
    fn default() -> Self {
        &OtherType
    }
}

#[derive(Default)]
struct OtherTypeWithParams<T, E>(T, E);

struct NotDefault;

impl NotDefault {
    pub fn default() -> u32 {
        0
    }
}

fn main() {
    u32::default();
    let _: Option<u32> = Option::default();
    let _ = Option::<u32>::default();
    let _: (usize,) = Default::default();
    let _: u32 = <_>::default();
    let _: (u32, _) = <(_, u32)>::default();
    let _: [u32; 10] = <[_; 10]>::default();
    let _: [u32; 10] = <[_; _]>::default();
    <()>::default();
    let _: [u32; 10] = Default::default();
    let _: [f32; 1000] = [Default::default(); 1000];
    let _ = <&str>::default();
    let _ = bool::default();
    let _ = <(u32, u32, bool)>::default();
    let _ = AtomicBool::default();
    let _ = AtomicI8::default();
    let _ = AtomicPtr::<u32>::default();
    let _ = AtomicPtr::<()>::default();
    let _ = AtomicU8::default();
    let _ = <&[u8]>::default();
    let _ = <&mut [u8]>::default();
    let _ = <[u8; 31]>::default();
    let _: (usize, OtherType) = Default::default();
    let _: &OtherType = <&OtherType>::default();
    let _: (usize, OtherTypeWithParams<u32, u16>) = Default::default();
    let _ = char::default();
    // Do not lint
    let _ = NotDefault::default();
    let _ = <(u32, u32, bool, &str)>::default();

    external! {
        u32::default();
        let _: Option<u32> = Option::default();
        let _: (usize,) = Default::default();
        <()>::default();
        let _: [u32; 10] = Default::default();
        let _: [f32; 1000] = [Default::default(); 1000];
        let _ = <&str>::default();
        let _ = bool::default();
        let _ = char::default();
    }
    with_span! {
        span
        u32::default();
        let _: Option<u32> = Option::default();
        let _: (usize,) = Default::default();
        <()>::default();
        let _: [u32; 10] = Default::default();
        let _: [f32; 1000] = [Default::default(); 1000];
        let _ = <&str>::default();
        let _ = bool::default();
        let _ = char::default();
    }
}
