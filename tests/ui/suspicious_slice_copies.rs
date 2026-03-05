#![warn(clippy::suspicious_slice_copies)]

const K: usize = 3;
const N: usize = 5;

fn arr_incr(arr: &mut [i32; K]) {
    for x in arr.iter_mut() {
        *x += 1;
    }
}

#[clippy::msrv = "1.92.0"]
fn test_msrv_1_92() {
    let mut arr: [i32; N] = [0; 5];
    let slice: &mut [i32] = &mut arr[..K];

    let mut_arr_ref: &mut [i32; K] = &mut slice.try_into().unwrap();
    //~^ suspicious_slice_copies

    let mut_arr_ref: &mut [i32; K] = &mut arr[..K].try_into().unwrap();
    //~^ suspicious_slice_copies

    arr_incr(&mut arr[..K].try_into().expect("never happen"));
    //~^ suspicious_slice_copies
}

#[clippy::msrv = "1.93.0"]
fn test_msrv_1_93() {
    let mut arr: [i32; N] = [0; 5];
    let slice: &mut [i32] = &mut arr[..K];

    let mut_arr_ref: &mut [i32; K] = &mut slice.try_into().unwrap();
    //~^ suspicious_slice_copies

    let mut_arr_ref: &mut [i32; K] = &mut arr[..K].try_into().unwrap();
    //~^ suspicious_slice_copies

    arr_incr(&mut arr[..K].try_into().expect("never happen"));
    //~^ suspicious_slice_copies
}

fn main() {
    let mut arr: [i32; N] = [0; 5];

    let arr_ref: &[i32; K] = &arr[..K].try_into().unwrap();
    let arr_mut_ref: &mut [i32; K] = (&mut arr[..K]).try_into().unwrap();

    let arr_copy: [i32; K] = arr[..K].try_into().unwrap();
}
