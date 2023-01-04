// run-rustfix
#![allow(unused)]
#![warn(clippy::byte_char_slice)]

fn main() {
    let bad = &[b'a', b'b', b'c'];
    let good = &[b'a', 0x42];
    let good = vec![b'a', b'a'];
    let good: u8 = [b'a', b'c'].into_iter().sum();
}
