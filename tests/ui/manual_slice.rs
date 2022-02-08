#![warn(clippy::manual_slice)]

fn main() {
    let mut arr: [u32; 3] = [1, 2, 3];

    let arr_slice = &arr[..];
    let mutable_arr_slice = &mut arr[..];

    let mut vec = vec![1, 2, 3];

    let vec_slice = &vec[..];
    let mutable_vec_slice = &mut vec[..];

    // Will not fire on any of these

    let partial_slice_1 = &arr[1..];
    let partial_slice_2 = &arr[..3];
    let partial_slice_3 = &arr[1..3];
    let partial_mut_slice_1 = &mut arr[1..];
    let partial_mut_slice_2 = &mut arr[..3];
    let partial_mut_slice_3 = &mut arr[1..3];

    let partial_slice_1 = &vec[1..];
    let partial_slice_2 = &vec[..3];
    let partial_slice_3 = &vec[1..3];
    let partial_mut_slice_1 = &mut vec[1..];
    let partial_mut_slice_2 = &mut vec[..3];
    let partial_mut_slice_3 = &mut vec[1..3];
}
