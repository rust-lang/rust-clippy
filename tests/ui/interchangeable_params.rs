#![warn(clippy::interchangeable_params)]
#![allow(clippy::needless_lifetimes, clippy::ptr_arg, unused)]
use std::fmt::{Debug, Display};
use std::sync::atomic::AtomicPtr;
//@no-rustfix: suggestions reference out of scope lifetimes/types and incomplete newtype specs
// like deref, from, borrow necessary for most uses.

// Standard library type
fn transfer(from: String, to: String) {
    //~^ interchangeable_params
    todo!();
}
fn samename(string: String, mystring: String) {}
//~^ interchangeable_params
fn fn0(a: String, b: i32) {}
// primitive type
fn fn2(a: u32, b: u64, c: &mut u32, d: i64) {}
//~^ interchangeable_params

fn fn3(param1: String, param2: i64) {}

struct LocalType(String);

fn fn4(first: LocalType, second: LocalType, third: String) {}
//~^ interchangeable_params

fn fn5<'a, 'b>(dogfood: &'a u32, catfood: u32, fishfood: &'b mut u32) {}
//~^ interchangeable_params
struct MyStruct {
    a: u32,
    b: u32,
}

// no errors because impl methods
impl MyStruct {
    fn mystruct1(self, first: MyStruct, second: MyStruct) {}

    fn mystruct2(self, first: MyStruct, second: &Self, third: &Self) {}

    fn mystruct3(first: MyStruct, second: u32, third: usize) {}
}

fn has_generics<T>(list: &[T], i: u32) -> &T {
    &list[0]
}

#[derive(Clone, Copy)]
pub struct RandomState {
    pub(crate) k0: u64,
    pub(crate) k1: u64,
    pub(crate) k2: u64,
    pub(crate) k3: u64,
}
// no error because impl methods
impl RandomState {
    fn write_usize(self, a: usize) {
        todo!();
    }
    fn write_u64(self, a: u64) {
        todo!();
    }
    fn finish(self) -> u64 {
        todo!();
    }
    fn new() -> RandomState {
        RandomState {
            k0: 0,
            k1: 0,
            k2: 0,
            k3: 0,
        }
    }
}

fn from_keys(a: &[u64; 4], b: &[u64; 4], c: usize) -> RandomState {
    RandomState::new()
}

fn unpack_alu(word: usize, second_word: usize, dst: *mut usize) {}
//~^ interchangeable_params

fn data_range(
    //~^ interchangeable_params
    // another comment
    data: &[u8], // test comment
    data_address: u64,
    range_address: u64,
    size: u64,
) -> Option<&[u8]> {
    None
}
unsafe fn shallow_clone_vec(
    //~^ interchangeable_params
    atom: &AtomicPtr<()>,
    ptr: *const (),
    buf: *mut u8,
    offset: *const u8,
    len: usize,
) -> usize {
    17
}

fn verify_affine_point_is_on_the_curve_scaled(
    //~^ interchangeable_params
    ops: &String,
    (x, y): (&u32, &u32),
    a_scaled: &u32,
    b_scaled: &u32,
) -> Result<(), std::fmt::Error> {
    Ok(())
}

// no error because nothing to name the i64 values.
fn unreachtest(_: i32, _: i64, _: i64, _: *mut u8) {
    unreachable!()
}

// no error because tuples don't count
fn memchr2(token: (u8, u8), slice: &[u8]) -> Option<usize> {
    Some(17)
}

const fn compute_bitmask(bit: u32, otherbit: u32) -> u32 {
    //~^ interchangeable_params
    (1 << bit) + otherbit
}

fn printer<T: Display>(pfirst: T, psecond: T) {
    todo!();
}

fn printer2<T, U>(p2first: T, p2second: U)
where
    T: Display + Debug,
    U: Display + Debug,
{
    todo!();
}

// do we need an async example???

fn main() {
    let mix = |l: u64, r: u64| todo!();
}
