//@aux-build:once_cell.rs

#![warn(clippy::once_cell_lazy)]
#![allow(static_mut_refs)]

use once_cell::sync::Lazy;

fn main() {}

static LAZY_FOO: Lazy<String> = Lazy::new(|| "foo".to_uppercase());
//~^ ERROR: this type has been superceded by `std::sync::LazyLock`
static LAZY_BAR: Lazy<String> = Lazy::new(|| {
    //~^ ERROR: this type has been superceded by `std::sync::LazyLock`
    let x = "bar";
    x.to_uppercase()
});
static LAZY_BAZ: Lazy<String> = { Lazy::new(|| "baz".to_uppercase()) };
//~^ ERROR: this type has been superceded by `std::sync::LazyLock`
static LAZY_QUX: Lazy<String> = {
    //~^ ERROR: this type has been superceded by `std::sync::LazyLock`
    if "qux".len() == 3 {
        Lazy::new(|| "qux".to_uppercase())
    } else if "qux".is_ascii() {
        Lazy::new(|| "qux".to_lowercase())
    } else {
        Lazy::new(|| "qux".to_string())
    }
};

fn non_static() {
    let _: Lazy<i32> = Lazy::new(|| 1);
    let _: Lazy<String> = Lazy::new(|| String::from("hello"));
    #[allow(clippy::declare_interior_mutable_const)]
    const DONT_DO_THIS: Lazy<i32> = Lazy::new(|| 1);
}

mod once_cell_lazy_with_fns {
    use super::Lazy; // nice

    static LAZY_FOO: Lazy<String> = Lazy::new(|| "foo".to_uppercase());
    //~^ ERROR: this type has been superceded by `std::sync::LazyLock`
    static LAZY_BAR: Lazy<String> = Lazy::new(|| "bar".to_uppercase());
    //~^ ERROR: this type has been superceded by `std::sync::LazyLock`
    static mut LAZY_BAZ: Lazy<String> = Lazy::new(|| "baz".to_uppercase());
    //~^ ERROR: this type has been superceded by `std::sync::LazyLock`

    fn calling_replaceable_fns() {
        let _ = Lazy::force(&LAZY_FOO);
        let _ = Lazy::force(&LAZY_BAR);
        unsafe {
            let _ = Lazy::force(&LAZY_BAZ);
        }
    }
}

#[clippy::msrv = "1.79"]
mod msrv_not_meet {
    use super::Lazy;

    static LAZY_FOO: Lazy<String> = Lazy::new(|| "foo".to_uppercase());
}
