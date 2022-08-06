#![warn(clippy::suspicious_arguments)]

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
}
