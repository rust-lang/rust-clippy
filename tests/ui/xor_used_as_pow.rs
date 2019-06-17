#![warn(clippy::xor_used_as_pow)]

fn main() {
    // These should succeed
    // With variables, it's not as clear whether the intention was exponentiation or not
    let x = 15;
    println!("{}", 2 ^ x);
    let y = 2;
    println!("{}", y ^ 16);

    // These should fail
    println!("{}", 2 ^ 16);
    println!("{}", 2 ^ 7);
    println!("{}", 9 ^ 3);
}
