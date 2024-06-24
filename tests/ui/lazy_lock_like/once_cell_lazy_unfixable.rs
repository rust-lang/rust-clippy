//@aux-build:once_cell.rs
//@no-rustfix

#![warn(clippy::once_cell_lazy)]
#![allow(static_mut_refs)]

use once_cell::sync::Lazy;

fn main() {}

static LAZY_FOO: Lazy<String> = Lazy::new(|| "foo".to_uppercase());
//~^ ERROR: this type has been superceded by `std::sync::LazyLock`
static LAZY_BAR: Lazy<String> = Lazy::new(|| "bar".to_uppercase());
//~^ ERROR: this type has been superceded by `std::sync::LazyLock`
static mut LAZY_BAZ: Lazy<String> = Lazy::new(|| "baz".to_uppercase());
//~^ ERROR: this type has been superceded by `std::sync::LazyLock`

fn calling_irreplaceable_fns() {
    let _ = Lazy::get(&LAZY_BAR);

    unsafe {
        let _ = Lazy::get_mut(&mut LAZY_BAZ);
        let _ = Lazy::force_mut(&mut LAZY_BAZ);
    }
}
