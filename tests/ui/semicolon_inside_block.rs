#![warn(clippy::semicolon_inside_block)]

unsafe fn f(arg: u32) {}

fn main() {
    let x = 32;

    unsafe { f(x) };
}

fn get_unit() {}

fn fooooo() {
    unsafe { f(32) }
}

fn moin() {
    println!("Hello")
}

fn hello() {
    get_unit()
}

fn basic101(x: i32) {
    let y: i32;
    y = x + 1
}

#[rustfmt::skip]
fn closure_error() {
    let _d = || {
        hello()
    };
}

fn my_own_block() {
    let x: i32;
    {
        let y = 42;
        x = y + 1;
        basic101(x)
    }
    assert_eq!(43, 43)
}

#[rustfmt::skip]
fn one_line_block() { println!("Foo") }
