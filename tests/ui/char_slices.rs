#![warn(clippy::char_slices)]

fn main() {
    let char_slice: &[char] = &['a', 'b'];
    takes_char_slice(char_slice);
    let char_slice2 = returns_char_slice();
}

fn takes_char_slice(char_slice: &[char]) {
    println!("{:?}", char_slice)
}

fn returns_char_slice() -> &'static [char] {
    &['c', 'd']
}
