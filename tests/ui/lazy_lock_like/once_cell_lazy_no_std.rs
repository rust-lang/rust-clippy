//@aux-build:once_cell.rs

#![warn(clippy::once_cell_lazy)]
#![no_std]

use once_cell::sync::Lazy;

fn main() {}

static LAZY_FOO: Lazy<usize> = Lazy::new(|| 42);
static LAZY_BAR: Lazy<i32> = Lazy::new(|| {
    let x: i32 = 0;
    x.saturating_add(100)
});
