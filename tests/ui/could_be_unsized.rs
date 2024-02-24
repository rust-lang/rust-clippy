#![warn(clippy::could_be_unsized)]

fn main() {}

// Lint
trait T1<T> {}

// Don't lint. `T` explicitly `Sized`
trait T2<T: Sized> {}

// Don't lint. `T` explicitly `Sized`
trait T3<T>
where
    T: Sized,
{
}

// Lint. `T` used by reference
trait T4<T> {
    fn foo(_: &T) -> &T;
}

// Don't lint.
trait T5<T> {
    fn foo(_: T) -> &'static T;
}

// Lint. `T` can be `?Sized`
trait T6<T, U, V: Sized> {
    fn foo(&self) -> &V;
    fn bar(_: &T) -> &U;
    fn baz(_: u32, _: &T) -> (u32, U);
}

// Don't lint.
trait T7<T>: T2<T> {}

trait Iterator {
    // Don't lint.
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
trait IntoIterator {
    // Don't lint.
    type Item;
    // Don't lint.
    type IntoIter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}

// Don't lint.
trait T8<T, U>
where
    T: IntoIterator<Item = U>,
{
}
