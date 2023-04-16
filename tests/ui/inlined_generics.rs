#![allow(unused)]
#![warn(clippy::inlined_generics)]

trait Foo<T> {
    fn foo(&self, t: T);

    #[inline]
    fn bar(&self, t: T) {
        self.foo(t);
    }
}

impl<T> Foo<T> for T {
    #[inline(always)]
    fn foo(&self, t: T) {}

    #[inline(never)] // This is ignored.
    fn bar(&self, t: T) {}
}

struct FooBar;

impl FooBar {
    #[inline]
    fn baz<T>(t: T) {}

    #[inline(never)]
    fn qux<T>(t: T) {}
}

#[inline]
fn foo<T: Copy>(t: T) {}
#[inline(always)]
fn bar<T>(t: T)
where
    T: Clone,
{
}
#[inline(never)] // Also ignored.
fn baz<T>(t: T) {}

fn main() {}
