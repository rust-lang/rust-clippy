//! **FAKE** once_cell crate.

pub mod sync {
    use std::marker::PhantomData;

    pub struct Lazy<T, F = fn() -> T> {
        cell: PhantomData<T>,
        init: F,
    }
    unsafe impl<T, F: Send> Sync for Lazy<T, F> {}
    impl<T, F> Lazy<T, F> {
        pub const fn new(f: F) -> Lazy<T, F> {
            Lazy {
                cell: PhantomData,
                init: f,
            }
        }

        pub fn into_value(this: Lazy<T, F>) -> Result<T, F> {
            unimplemented!()
        }

        pub fn force(_this: &Lazy<T, F>) -> &T {
            unimplemented!()
        }

        pub fn force_mut(_this: &mut Lazy<T, F>) -> &mut T {
            unimplemented!()
        }

        pub fn get(_this: &Lazy<T, F>) -> Option<&T> {
            unimplemented!()
        }

        pub fn get_mut(_this: &mut Lazy<T, F>) -> Option<&mut T> {
            unimplemented!()
        }
    }
}
