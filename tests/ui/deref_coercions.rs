#![warn(clippy::deref_coercions)]
use std::ops::Deref;

fn main() {
    let x = &Box::new(true);
    let _: &bool = x;
}