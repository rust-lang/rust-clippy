#![allow(unused)]
#![warn(clippy::redundant_param_refs)]

fn main() {}

use std::fmt::Display;
use std::io;
use std::io::prelude::*;

struct S {}

// ------- Should match -------

fn f1<R: io::Read>(r: &mut R)
where
    R: Write,
{
}
fn f2<R>(r: &mut R)
where
    R: Read,
{
}
fn f3<RW: io::Read>(r: &mut RW)
where
    RW: Write,
{
}
fn f4<RW: Read + Write>(r: &mut RW) {}
fn f5<T: Display>(a: &T) {}

impl S {
    fn m1<R: Read>(&self, r: &mut R) {}
}

trait Tr {
    fn tm1<R: Read>(r: &mut R) -> usize;
    fn tm2<R: Read>(r: &mut R) {}
}

// ----- Should not match -----

fn g1<R: Read>(r: R) {}
fn g2<R: Read>(r: &R) {}
fn g3<R: Read + ToString>(r: &mut R) {}
fn g4<R>(r: &mut R)
where
    R: Read + ToString,
{
}
fn g5<R: Read>(r: &mut R)
where
    R: ToString,
{
}
fn g6(r: &mut usize) {}
fn g7(r: &mut dyn Read) {}
fn g8<T>(r: &T) {}

impl S {
    fn n1(&self) {}
}

impl Tr for S {
    fn tm1<R: Read>(r: &mut R) -> usize {
        1
    }
}
