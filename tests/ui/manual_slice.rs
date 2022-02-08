#![warn(clippy::manual_slice)]

fn main() {
    let mut arr: [u32; 1] = [1];
    let slice = &arr[..];
    let mutable_slice = &mut arr[..];
}
