#![warn(clippy::slice_iter_any)]

fn main() {
    let vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient

    let vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.contains(&4);
    // no error, because it uses `contains()`

    let vec: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient

    let values: [u8; 6] = [3, 14, 15, 92, 6, 5];
    let _ = values.iter().any(|&v| v == 10);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient

    let values: [u8; 6] = [3, 14, 15, 92, 6, 5];
    let _ = values.iter().any(|&v| v > 10);
    // no error, because this `any()` is not for equality

    let vec: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v % 2 == 0);
    // no error, because this `iter().any()` can't be replaced with `contains()` simply

    let num = 14;
    let values: [u8; 6] = [3, 14, 15, 92, 6, 5];
    let _ = values.iter().any(|&v| v == num);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient

    let values: [u8; 6] = [3, 14, 15, 92, 6, 5];
    let _ = values.iter().any(|v| *v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient
}

fn foo(values: &[u8]) -> bool {
    values.iter().any(|&v| v == 10)
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient
}

fn bar(values: [u8; 3]) -> bool {
    values.iter().any(|&v| v == 10)
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient
}

#[clippy::msrv = "1.83"]
fn u8_slice_and_msrv_is_1_83_0() {
    let vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient
}

#[clippy::msrv = "1.83"]
fn u32_slice_and_msrv_is_1_83_0() {
    let vec: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    // Shouldn't trigger the performance lint because the MSRV is 1.83.0 and this is a u32 slice
}

#[clippy::msrv = "1.84"]
fn u8_slice_and_msrv_is_1_84_0() {
    let vec: Vec<u8> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient
}

#[clippy::msrv = "1.84"]
fn u32_slice_and_msrv_is_1_84_0() {
    let vec: Vec<u32> = vec![1, 2, 3, 4, 5, 6];
    let values = &vec[..];
    let _ = values.iter().any(|&v| v == 4);
    //~^ ERROR: using `contains()` instead of `iter().any()` is more efficient
}
