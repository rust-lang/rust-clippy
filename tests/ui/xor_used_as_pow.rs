#![warn(clippy::xor_used_as_pow)]

fn main() {
    println!("{}", 2 ^ 16);
    // Should be allowed
    let x = 16;
    println!("{}", 2 ^ x);
    let y = 2;
    println!("{}", y ^ 16);
}
