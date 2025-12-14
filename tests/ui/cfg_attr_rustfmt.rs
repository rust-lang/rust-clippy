#![warn(clippy::deprecated_cfg_attr)]

// #![rusfmt::skip] requires https://github.com/rust-lang/rust/issues/54726
#![cfg_attr(rustfmt, rustfmt_skip)]

#[rustfmt::skip]
fn foo(
) {}

#[cfg_attr(rustfmt, rustfmt_skip)]
//~^ deprecated_cfg_attr
fn outer() {

    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    let _ = 1;

    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    foo();
}

struct S {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    field: String
}

impl S {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    fn g() {}
}

trait A {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    fn f();
}

impl A for S {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    fn f() {}
}

enum E {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    Variant {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        //~^ deprecated_cfg_attr
        field: String,
    }
}

unsafe extern "C" {
    #[cfg_attr(rustfmt, rustfmt_skip)]
    //~^ deprecated_cfg_attr
    fn external();
}

fn invalid() {
    #![cfg_attr(rustfmt, rustfmt_skip)]

    #[cfg_attr(rustfmt, rustfmt::skip)]
    {}

    #[cfg_attr(rustfmt, rustfmt_skip)]
    foo()
}

#[clippy::msrv = "1.29"]
#[cfg_attr(rustfmt, rustfmt::skip)]
fn msrv_1_29() {}

#[clippy::msrv = "1.30"]
#[cfg_attr(rustfmt, rustfmt::skip)]
//~^ deprecated_cfg_attr
fn msrv_1_30() {}
