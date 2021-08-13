// run-rustfix
#![warn(clippy::semicolon_outside_block)]
#![allow(dead_code)]

unsafe fn f(_arg: u32) {}

#[rustfmt::skip]
fn main() {
    let x = 32;

    unsafe { f(x); }
}

fn foo() {
    let x = 32;

    unsafe {
        f(x);
    }
}

fn bar() {
    let x = 32;

    unsafe {
        let _this = 1;
        let _is = 2;
        let _a = 3;
        let _long = 4;
        let _list = 5;
        let _of = 6;
        let _variables = 7;
        f(x);
    };
}

fn get_unit() {}

#[rustfmt::skip]
fn closure_error() {
    let _d = || {
        get_unit();
    };
}

fn my_own_block() {
    let x: i32;
    {
        let y = 42;
        x = y + 1;
        get_unit();
    }
    assert_eq!(x, 43)
}

// This is all ok

fn just_get_unit() {
    get_unit();
}

fn test_if() {
    if 1 > 2 {
        get_unit();
    } else {
        println!("everything alright!");
    }
}

fn test_for() {
    for _project in &[
        "clippy_workspace_tests",
        "clippy_workspace_tests/src",
        "clippy_workspace_tests/subcrate",
        "clippy_workspace_tests/subcrate/src",
        "clippy_dev",
        "clippy_lints",
        "clippy_utils",
        "rustc_tools_util",
    ] {
        get_unit();
    }

    get_unit();
}
