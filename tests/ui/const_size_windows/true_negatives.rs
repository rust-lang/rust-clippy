//@check-pass
#![warn(clippy::const_size_windows)]

macro_rules! number {
    () => {
        (0..100).count()
    };
}

macro_rules! as_window_pairs {
    ($slice:expr) => {{
        let s: &[_] = $slice;
        s.windows(2)
    }};
}

fn get_number() -> usize {
    (0..100).filter(|num| num % 2 == 0).count()
}

fn slice_windows_variable_size(slice: &[u8], size: usize) {
    for window in slice.windows(size) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn slice_windows_fn_variable_size(slice: &[u8]) {
    for window in slice.windows(get_number()) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn slice_windows_macro_variable_size(slice: &[u8]) {
    for window in slice.windows(number!()) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn vec_windows_variable_size(vector: Vec<u8>, size: usize) {
    for window in vector.windows(size) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn array_windows_variable_size(array: [u8; 3], size: usize) {
    for window in array.windows(size) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn array_generic_length_windows_variable_size<const LENGTH: usize>(array: [u8; LENGTH], size: usize) {
    for window in array.windows(size) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn inline_vec_windows_variable_size(size: usize) {
    #[expect(clippy::useless_vec)]
    for window in vec![1, 2, 3].windows(size) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn inline_array_windows_variable_size(size: usize) {
    for window in [1, 2, 3].windows(size) {
        let window: &[i32] = window;
        println!("{}", window[0]);
    }
}

fn inline_slice_windows_variable_size(size: usize) {
    for window in [1, 2, 3, 4, 5][..=2].windows(size) {
        let window: &[i32] = window;
        println!("{}", window[0]);
    }
}

fn into_slice_windows_variable_size<'a>(into_slice: impl Into<&'a [u8]>, size: usize) {
    for window in into_slice.into().windows(size) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn deref_slice_windows_variable_size(deref_slice: impl std::ops::Deref<Target = [u8]>, size: usize) {
    for window in (*deref_slice).windows(size) {
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn macro_containing_slice_windows_literal_size(slice: &[u8]) {
    for pair in as_window_pairs!(slice) {
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn main() {}
