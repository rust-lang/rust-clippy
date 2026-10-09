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
    // would be invalid: `slice.array_windows::<size>()`
    for window in slice.windows(size) {
        println!("{}", window[0]);
    }
}

fn slice_windows_fn_variable_size(slice: &[u8]) {
    // would be invalid: `vector.array_windows::<{ get_number() }>()`
    for window in slice.windows(get_number()) {
        println!("{}", window[0]);
    }
}

fn slice_windows_macro_variable_size(slice: &[u8]) {
    // would be invalid: `vector.array_windows::<{ number!() }>()`
    for window in slice.windows(number!()) {
        println!("{}", window[0]);
    }
}

fn vec_windows_variable_size(vector: Vec<u8>, size: usize) {
    // would be invalid: `vector.array_windows::<size>()`
    for window in vector.windows(size) {
        println!("{}", window[0]);
    }
}

fn array_windows_variable_size(array: [u8; 3], size: usize) {
    // would be invalid: `array.array_windows::<size>()`
    for window in array.windows(size) {
        println!("{}", window[0]);
    }
}

fn array_generic_length_windows_variable_size<const LENGTH: usize>(array: [u8; LENGTH], size: usize) {
    // would be invalid: `array.array_windows::<size>()`
    for window in array.windows(size) {
        println!("{}", window[0]);
    }
}

fn inline_vec_windows_variable_size(size: usize) {
    // would be invalid: `vec![1, 2, 3].array_windows::<size>()`
    #[expect(clippy::useless_vec)]
    for window in vec![1, 2, 3].windows(size) {
        println!("{}", window[0]);
    }
}

fn inline_array_windows_variable_size(size: usize) {
    // would be invalid: `[1, 2, 3].array_windows::<size>()`
    for window in [1, 2, 3].windows(size) {
        println!("{}", window[0]);
    }
}

fn inline_slice_windows_variable_size(size: usize) {
    // would be invalid: `[1, 2, 3, 4, 5][..=2].array_windows::<size>()`
    for window in [1, 2, 3, 4, 5][..=2].windows(size) {
        println!("{}", window[0]);
    }
}

fn into_slice_windows_variable_size<'a>(into_slice: impl Into<&'a [u8]>, size: usize) {
    // would be invalid: `into_slice.into().array_windows::<size>()`
    for window in into_slice.into().windows(size) {
        println!("{}", window[0]);
    }
}

fn deref_slice_windows_variable_size(deref_slice: impl std::ops::Deref<Target = [u8]>, size: usize) {
    // would be invalid: `(*deref_slice).array_windows::<size>()`
    for window in (*deref_slice).windows(size) {
        println!("{}", window[0]);
    }
}

fn macro_containing_slice_windows_literal_size(slice: &[u8]) {
    // we don't want to lint the macro
    for pair in as_window_pairs!(slice) {
        println!("{} {}", pair[0], pair[1]);
    }
}

fn const_generic_param_computation<const T: usize>(slice: &[u8]) {
    // would be invalid: `slice.array_windows::<{ T + 1 }>()`
    for window in slice.windows(T + 1) {
        println!("{}", window[0]);
    }
}

fn const_generic_param_from_generic_fn_result<T>(slice: &[u8]) {
    // would be invalid: `slice.array_windows::<{ std::mem::size_of::<T>() }>()`
    for window in slice.windows(std::mem::size_of::<T>()) {
        println!("{}", window[0]);
    }
}

#[clippy::msrv = "1.93"]
fn before_array_windows_stabilization(slice: &[u8]) {
    // would not compile at 1.93: `slice.array_windows::<2>()`
    for pair in slice.windows(2) {
        println!("{} {}", pair[0], pair[1]);
    }
}

fn main() {}
