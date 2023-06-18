//@run-rustfix
//@aux-build:proc_macros.rs:proc-macro
#![allow(clippy::no_effect, unused)]
#![warn(clippy::trivial_default_constructed_types)]

#[macro_use]
extern crate proc_macros;

fn main() {
    u32::default();
    let x: Option<u32> = Option::default();
    let y: (usize,) = Default::default();
    <()>::default();
    let x: [u32; 10] = Default::default();
    let x: [f32; 1000] = [Default::default(); 1000];
    let x = <&str>::default();
    let x = bool::default();
    // Do not lint
    let x = char::default();

    external! {
        u32::default();
        let x: Option<u32> = Option::default();
        let y: (usize,) = Default::default();
        <()>::default();
        let x: [u32; 10] = Default::default();
        let x: [f32; 1000] = [Default::default(); 1000];
        let x = <&str>::default();
        let x = bool::default();
        let x = char::default();
    }
    with_span! {
        span
        u32::default();
        let x: Option<u32> = Option::default();
        let y: (usize,) = Default::default();
        <()>::default();
        let x: [u32; 10] = Default::default();
        let x: [f32; 1000] = [Default::default(); 1000];
        let x = <&str>::default();
        let x = bool::default();
        let x = char::default();
    }
}
