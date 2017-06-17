#![feature(inclusive_range_syntax)]
#![feature(plugin)]
#![plugin(clippy)]


#[warn(range_zip_with_len)]
fn main() {
    let v1 = vec![1,2,3];
    let v2 = vec![4,5];
    let _x = v1.iter().zip(0..v1.len());
    let _y = v1.iter().zip(0..v2.len()); // No error
}
