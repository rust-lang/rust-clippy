// edition:2015
#![warn(clippy::suspicious_arguments)]
#![allow(anonymous_parameters, clippy::no_effect)]

trait AnonymousArgs {
    fn scale(&self, usize, usize) {}
}

fn scale_this<A: AnonymousArgs>(value: A) {
    // We don't expect a lint, we just don't want an ICE here.
    let width = 1;
    let height = 2;
    value.scale(width, height);
    A::scale(&value, width, height);
}

fn resize(width: usize, height: usize) {}

struct Bitmap;

impl Bitmap {
    fn new(width: usize, height: usize) -> Self {
        Bitmap
    }
}

struct TupleStruct(usize, usize);

enum TupleEnum {
    Hello,
    World(usize, usize),
}

#[derive(Default)]
struct Dimensions {
    width: usize,
    height: usize,
}

fn function_names() {
    fn height() -> usize { 0 }
    fn width() -> usize { 0 }

    resize(height(), width());
}

fn pathed_function_names() {
    mod uwu {
        pub fn height() -> usize { 0 }
        pub fn width() -> usize { 0 }
    }

    resize(uwu::height(), uwu::width());
}

fn variable_names() {
    let width = 0;
    let height = 0;

    resize(height, width);
    Bitmap::new(height, width);
}

#[rustfmt::ignore]
fn struct_names() {
    let dims = Dimensions::default();
    resize(dims.height, dims.width);

    resize(
        dims
        .height,
        // Very long!
        dims
        .width);
}

fn should_not_lint() {
    let width = 0;
    let height = 0;
    TupleStruct(width, height);
    TupleEnum::World(width, height);

    resize(0, width);
    resize(height, 0);
    resize(height, height);
    resize(width, width);
}

fn cross_crate() {
    let f = vec![0_u8];
    let iterable = |_| { true };

    itertools::all(f, iterable);
}

fn cross_std() {
    let mut xvalue = 42;
    let mut yvalue = 42;
    let x = &mut xvalue;
    let y = &mut yvalue;

    std::mem::swap(y, x);
}

fn varargs() {
    extern "C" {
        fn test_var_args(width: usize, height: usize, ...);
    }
    
    if false {
        unsafe {
            let width = 0;
            let height = 0;
            let not_foo = 0;
            test_var_args(height, width, not_foo, width, height);
        }
    }
}


fn main() {
    function_names();
    variable_names();
    struct_names();
    should_not_lint();
    cross_crate();
    cross_std();
}
