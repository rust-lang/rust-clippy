#![warn(clippy::semicolon_outside_block)]

unsafe fn f(arg: u32) {}

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

fn moin() {
    {
        let _u = get_unit();
        println!("Hello");
    }
}

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
    assert_eq!(43, 43)
}

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
