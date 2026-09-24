#![allow(clippy::box_collection, clippy::redundant_slicing)]
#![warn(clippy::manual_as_slice)]

mod nested_1 {
    pub(crate) mod nested_2 {
        pub(crate) const SOME_VALUE: [u8; 4] = [1, 2, 3, 4];
    }
}

fn main() {
    let array: [u8; 4] = [0; 4];
    let slice: &[u8] = &array[..];
    //~^ manual_as_slice

    let mut array: [u8; 4] = [0; 4];
    let mut slice: &[u8] = &mut array[..];
    //~^ manual_as_slice

    let slice: &[u8] = &nested_1::nested_2::SOME_VALUE[..];
    //~^ manual_as_slice

    let slice = b"foo";
    let slice: &[u8] = &slice[..];
    //~^ manual_as_slice

    let slice = &[1, 2, 3, 4];
    let slice: &[u8] = &slice[..];
    //~^ manual_as_slice

    macro_rules! perform_the_slice {
        ($a:expr) => {
            &$a[..]
        };
    }

    perform_the_slice!([1, 2, 3]);

    let slice: &[u8] = &vec![1, 2, 3][..];
    //~^ manual_as_slice

    struct Object {
        field: Vec<u8>,
    }

    let object = Object { field: vec![1, 2, 3] };
    let slice: &[u8] = &object.field[..];
    //~^ manual_as_slice

    // sized types in a box where the contents of the box is slice-like, shoudl emit the manual_as_slice
    struct SizedBox(Box<Vec<u8>>);
    let object = SizedBox(Box::default());
    let slice: &[u8] = &object.0[..];
    //~^ manual_as_slice

    struct UnsizedBox(Box<[u8]>);
    let object = UnsizedBox(Box::new([]));
    // unsized slices don't have a as_slice implementations, so don't suggest the lint
    let slice: &[u8] = &object.0[..];

    // FIXME: suggest `.as_str()` here
    let slice = &"foo"[..];

    struct Count;

    impl<R: std::ops::RangeBounds<()>> std::ops::Index<R> for Count {
        type Output = ();

        fn index(&self, _: R) -> &Self::Output {
            &()
        }
    }

    // don't suggest `.as_slice()` on non-std types, as they might not have the method,
    // or it might do something different
    let count = &Count[..];

    // don't suggest `[].as_slice()`, as it would be too verbose
    let empty: &[u8] = &[][..];
}
