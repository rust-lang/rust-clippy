#![warn(clippy::not_exhaustive_enough)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::never_loop)]

use std::io::ErrorKind;

#[non_exhaustive]
pub enum E {
    First,
    Second,
    Third,
}

#[non_exhaustive]
pub enum K {
    First(String),
    Second(u32, u32),
    Third(String)
}

#[derive(Default)]
#[non_exhaustive]
pub struct S {
    pub a: i32,
    pub b: i32,
    pub c: i32,
}

#[derive(Default)]
#[non_exhaustive]
pub struct T(pub i32, pub i32, pub i32);

#[derive(Default)]
#[non_exhaustive]
pub struct W(pub i32, pub i32, i32);

fn main() {
    //////// Enum

    let e = E::First;

    match e {
        E::First => {},
        E::Second => {},
        _ => {},
    }

    //
    let example = "Example".to_string();
    let k = K::First(example);

    match k {
        K::First(..) => {},
        K::Second(..) => {},
        _ => {},
    }

    //////// Struct

    let S { a: _, b: _, .. } = S::default();

    match S::default() {
        S { a: 42, b: 21, .. } => {},
        S { a: _, b: _, .. } => {},
    }

    if let S { a: 42, b: _, .. } = S::default() {}

    let v = vec![S::default()];

    for S { a: _, b: _, .. } in v {}

    while let S { a: 42, b: _, .. } = S::default() {
        break;
    }

    pub fn take_s(S { a, b, .. }: S) -> (i32, i32) {
        (a, b)
    }

    //////// Tuple Struct

    let T { 0: _, 1: _, .. } = T::default();

    match T::default() {
        T { 0: 42, 1: 21, .. } => {},
        T { 0: _, 1: _, .. } => {},
    }

    if let T { 0: 42, 1: _, .. } = T::default() {}

    let v = vec![T::default()];
    for T { 0: _, 1: _, .. } in v {}

    while let T { 0: 42, 1: _, .. } = T::default() {
        break;
    }

    pub fn take_t(T { 0: _, 1: _, .. }: T) -> (i32, i32) {
        (0, 1)
    }

    //

    let W { 0: _, 1: _, .. } = W::default();

    match W::default() {
        W { 0: 42, 1: 21, .. } => {},
        W { 0: _, 1: _, .. } => {},
    }

    if let W { 0: 42, 1: _, .. } = W::default() {}

    let m = vec![W::default()];
    for W { 0: _, 1: _, .. } in m {}

    while let W { 0: 42, 1: _, .. } = W::default() {
        break;
    }

    pub fn take_w(W { 0: _, 1: _, .. }: W) -> (i32, i32) {
        (0, 1)
    }

    // Enum - Another Crate
    let error_kind = ErrorKind::ConnectionReset;

    match error_kind {
        ErrorKind::NotFound => {},
        ErrorKind::PermissionDenied => {},
        ErrorKind::ConnectionRefused => {},
        ErrorKind::ConnectionReset => {},
        ErrorKind::ConnectionAborted => {},
        ErrorKind::NotConnected => {},
        ErrorKind::AddrInUse => {},
        ErrorKind::AddrNotAvailable => {},
        ErrorKind::BrokenPipe => {},
        ErrorKind::AlreadyExists => {},
        _ => {},
    }

    // Struct - Another Crate
    // todo
}
