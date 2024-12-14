#![warn(clippy::slice_iter_any)]

fn main() {
    let vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` on u8/i8 slices is more efficient

    let vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.contains(&4);
    // no error, because it uses `contains()`

    let vec: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more readable

    let values: [u8; 6] = [3, 14, 15, 92, 6, 5];
    let _ = values.iter().any(|&v| v == 10);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more readable

    let values: [u8; 6] = [3, 14, 15, 92, 6, 5];
    let _ = values.iter().any(|&v| v > 10);
    // no error, because this `any()` is not for equality
}

fn foo(values: &[u8]) -> bool {
    values.iter().any(|&v| v == 10)
    //~^ ERROR: using `contains()` instead of `iter().any()` on u8/i8 slices is more efficient
}

fn bar(values: [u8; 3]) -> bool {
    values.iter().any(|&v| v == 10)
    //~^ ERROR: using `contains()` instead of `iter().any()` is more readable
}
