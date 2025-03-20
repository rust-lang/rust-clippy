//@no-rustfix
#![warn(clippy::unused_enumerate_value)]

fn main() {
    struct Length(usize);

    impl IntoIterator for Length {
        type Item = usize;
        type IntoIter = std::iter::Once<usize>;

        fn into_iter(self) -> Self::IntoIter {
            std::iter::once(self.0)
        }
    }

    impl Length {
        fn len(&self) -> usize {
            self.0
        }
    }

    let length = Length(3);
    for (index, _) in length.into_iter().enumerate() {
        //~^ unused_enumerate_value
        todo!();
    }
}
