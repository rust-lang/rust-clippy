#![warn(clippy::const_size_windows)]

macro_rules! two {
    () => {
        2usize
    };
}

const fn get_two() -> usize {
    2
}

fn slice_windows_literal_size(slice: &[u8]) {
    for pair in slice.windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn slice_windows_literal_size_three(slice: &[u8]) {
    for triplet in slice.windows(3) {
        //~^ const_size_windows
        let triplet: &[u8] = triplet;
        println!("{} {} {}", triplet[0], triplet[1], triplet[2]);
    }
}

fn slice_windows_const_size(slice: &[u8]) {
    const TWO: usize = 2;
    for pair in slice.windows(TWO) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn slice_windows_param_const_size<const SIZE: usize>(slice: &[u8]) {
    for window in slice.windows(SIZE) {
        //~^ const_size_windows
        let window: &[u8] = window;
        println!("{}", window[0]);
    }
}

fn slice_windows_const_fn_size(slice: &[u8]) {
    for pair in slice.windows(get_two()) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn slice_windows_inline_const_macro_size(slice: &[u8]) {
    for pair in slice.windows(two!()) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn vec_windows_literal_size(vector: Vec<u8>) {
    for pair in vector.windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn array_windows_literal_size(array: [u8; 3]) {
    for pair in array.windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn array_generic_length_windows_literal_size<const LENGTH: usize>(array: [u8; LENGTH]) {
    for pair in array.windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn inline_vec_windows_literal_size() {
    #[expect(clippy::useless_vec)]
    for pair in vec![1, 2, 3].windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn inline_array_windows_literal_size() {
    for pair in [1, 2, 3].windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn inline_slice_windows_literal_size() {
    for pair in [1, 2, 3, 4, 5][..=2].windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn into_slice_windows_literal_size<'a>(into_slice: impl Into<&'a [u8]>) {
    for pair in into_slice.into().windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn deref_slice_windows_literal_size(deref_slice: impl std::ops::Deref<Target = [u8]>) {
    for pair in (*deref_slice).windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn deref_coerced_slice_windows_literal_size(deref_slice: impl std::ops::Deref<Target = [u8]>) {
    for pair in deref_slice.windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}

fn shadowed_destruct_variables() {
    let vec: Vec<u8> = vec![1, 2, 3];
    let (left, right) = (0, 1);
    let (x, y) = (0, 1);

    for pair in vec.windows(2) {
        //~^ const_size_windows
        let pair: &[u8] = pair;
        println!("{} {}", pair[0], pair[1]);
    }
}
