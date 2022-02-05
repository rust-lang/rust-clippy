#![warn(clippy::map_then_identity_transformer)]
#![allow(clippy::map_identity)]

fn main() {
    let a = [1, 2, 3];

    let _ = a.into_iter().map(|x| x > 0).all(|x| x);
    let _ = a.into_iter().map(|x| x + x > 0).all(|x| x);
}
