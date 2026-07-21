#![warn(clippy::const_size_windows)]

// UTIL

macro_rules! two {
    () => {
        2usize
    };
}

macro_rules! number {
    () => {
        (0..100).count()
    };
}

const fn get_two() -> usize {
    2
}

fn get_number() -> usize {
    (0..100).filter(|num| num % 2 == 0).count()
}

// TRUE POSITIVES

fn slice_windows_literal_size(slice: &[u8]) {
    for pair in slice.windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn slice_windows_literal_size_three(slice: &[u8]) {
    for triplet in slice.windows(3) {
        //~^ const_size_windows
        println!("{} {} {}", triplet[0], triplet[1], triplet[2]);
    }
}

fn slice_windows_const_size(slice: &[u8]) {
    const TWO: usize = 2;
    for pair in slice.windows(TWO) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn slice_windows_param_const_size<const SIZE: usize>(slice: &[u8]) {
    for window in slice.windows(SIZE) {
        //~^ const_size_windows
        println!("{}", window[0]);
    }
}

fn slice_windows_const_fn_size(slice: &[u8]) {
    for pair in slice.windows(get_two()) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn slice_windows_inline_const_macro_size(slice: &[u8]) {
    for pair in slice.windows(two!()) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn vec_windows_literal_size(vector: Vec<u8>) {
    for pair in vector.windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn array_windows_literal_size(array: [u8; 3]) {
    for pair in array.windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn array_generic_length_windows_literal_size<const LENGTH: usize>(array: [u8; LENGTH]) {
    for pair in array.windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

#[allow(clippy::useless_vec)]
fn inline_vec_windows_literal_size() {
    for pair in vec![1, 2, 3].windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn inline_array_windows_literal_size() {
    for pair in [1, 2, 3].windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn inline_slice_windows_literal_size() {
    for pair in [1, 2, 3, 4, 5][..=2].windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn into_slice_windows_literal_size<'a>(into_slice: impl Into<&'a [u8]>) {
    for pair in into_slice.into().windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn deref_slice_windows_literal_size(deref_slice: impl std::ops::Deref<Target = [u8]>) {
    for pair in (*deref_slice).windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn shadowed_array_windows_method() {
    trait WindowsEmojiProvider {
        fn array_windows<const LENGTH: usize>(&self) -> [char; LENGTH] {
            ['🪟'; LENGTH]
        }
    }

    impl WindowsEmojiProvider for Vec<f32> {}

    let vec: Vec<f32> = vec![1.0, 2.0, 3.0];
    for pair in vec.windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

#[allow(clippy::ptr_arg)]
fn shadowed_array_windows_method_borrowed(vec: &Vec<f32>) {
    trait WindowsEmojiProvider {
        fn array_windows<const LENGTH: usize>(&self) -> [char; LENGTH] {
            ['🪟'; LENGTH]
        }
    }

    impl WindowsEmojiProvider for Vec<f32> {}

    for pair in vec.windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

fn shadowed_destruct_variables() {
    let vec: Vec<u8> = vec![1, 2, 3];
    let (left, right) = (0, 1);
    let (x, y) = (0, 1);

    for pair in vec.windows(2) {
        //~^ const_size_windows
        println!("{} {}", pair[0], pair[1]);
    }
}

// TRUE NEGATIVES

fn slice_windows_variable_size(slice: &[u8], size: usize) {
    for window in slice.windows(size) {
        println!("{}", window[0]);
    }
}

fn slice_windows_fn_variable_size(slice: &[u8]) {
    for window in slice.windows(get_number()) {
        println!("{}", window[0]);
    }
}

fn slice_windows_macro_variable_size(slice: &[u8]) {
    for window in slice.windows(number!()) {
        println!("{}", window[0]);
    }
}

fn vec_windows_variable_size(vector: Vec<u8>, size: usize) {
    for window in vector.windows(size) {
        println!("{}", window[0]);
    }
}

fn array_windows_variable_size(array: [u8; 3], size: usize) {
    for window in array.windows(size) {
        println!("{}", window[0]);
    }
}

fn array_generic_length_windows_variable_size<const LENGTH: usize>(array: [u8; LENGTH], size: usize) {
    for window in array.windows(size) {
        println!("{}", window[0]);
    }
}

#[allow(clippy::useless_vec)]
fn inline_vec_windows_variable_size(size: usize) {
    for window in vec![1, 2, 3].windows(size) {
        println!("{}", window[0]);
    }
}

fn inline_array_windows_variable_size(size: usize) {
    for window in [1, 2, 3].windows(size) {
        println!("{}", window[0]);
    }
}

fn inline_slice_windows_variable_size(size: usize) {
    for window in [1, 2, 3, 4, 5][..=2].windows(size) {
        println!("{}", window[0]);
    }
}

fn into_slice_windows_variable_size<'a>(into_slice: impl Into<&'a [u8]>, size: usize) {
    for window in into_slice.into().windows(size) {
        println!("{}", window[0]);
    }
}

fn deref_slice_windows_variable_size(deref_slice: impl std::ops::Deref<Target = [u8]>, size: usize) {
    for window in (*deref_slice).windows(size) {
        println!("{}", window[0]);
    }
}

fn overloaded_windows_method() {
    trait WindowsEmojiProvider {
        fn windows(&self, size: usize) -> impl Iterator<Item = char> {
            std::iter::repeat_n('🪟', size)
        }
    }

    impl WindowsEmojiProvider for Vec<f64> {}

    let vec: Vec<f64> = vec![1.0, 2.0, 3.0];
    for emoji in vec.windows(2) {
        println!("{emoji}");
    }
}

macro_rules! as_window_pairs {
    ($slice:expr) => {{
        let s: &[_] = $slice;
        s.windows(2)
    }};
}

fn macro_containing_slice_windows_literal_size(slice: &[u8]) {
    for pair in as_window_pairs!(slice) {
        println!("{} {}", pair[0], pair[1]);
    }
}

fn main() {}
