//@ check-pass

#![warn(clippy::useless_asref)]
#![expect(clippy::needless_lifetimes)]

trait Trait {
    fn as_ptr(&self) -> *const u8;
}

impl<'a> Trait for &'a [u8] {
    fn as_ptr(&self) -> *const u8 {
        self.as_ref().as_ptr()
    }
}

fn main() {}
