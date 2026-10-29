//@ check-pass

#![warn(clippy::useless_asref)]
#![expect(clippy::missing_return_type, clippy::needless_lifetimes)]

trait Trait {
    fn as_ptr(&self);
}

impl<'a> Trait for &'a [u8] {
    fn as_ptr(&self) {
        self.as_ref().as_ptr();
    }
}

fn main() {}
