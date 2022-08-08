// edition:2015
#![warn(clippy::suspicious_arguments)]
#![allow(anonymous_parameters)]

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

fn main() {
    let width = 0;
    let height = 0;

    resize(height, width);
    Bitmap::new(height, width);

    resize(0, width);
    resize(height, 0);
    resize(height, height);
    resize(width, width);

    let f = vec![0_u8];
    let iterable = |_| { true };

    itertools::all(f, iterable);

    let mut xvalue = 42;
    let mut yvalue = 42;
    let x = &mut xvalue;
    let y = &mut yvalue;

    std::mem::swap(y, x);
    
}
