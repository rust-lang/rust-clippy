#![warn(clippy::not_exhaustive_enough)]

use std::io::ErrorKind;

#[non_exhaustive]
pub enum E {
    First,
    Second,
    Third,
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

fn main() {
    //////// Enum

    let e = E::First;

    match e {
        E::First => {},
        E::Second => {},
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

    //////// Tuple Struct

    let T { 0: _, 1: _, .. } = T::default();

    match T::default() {
        T { 0: 42, 1: 21, .. } => {},
        T { 0: _, 1: _, .. } => {},
    }

    if let T { 0: 42, 1: _, .. } = T::default() {}

    let v = vec![T::default()];
    for T { 0: _, 1: _, .. } in v {}

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
