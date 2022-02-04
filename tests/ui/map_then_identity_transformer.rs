#![warn(clippy::map_then_identity_transformer)]
#![allow(clippy::map_identity)]

fn main() {
    let a = [1, 2, 3].into_iter();
    let _ = a.map(|x| x > 0).all(|x| x);
    
}
