#![warn(clippy::useless_vec)]
#![allow(clippy::collection_is_never_read)]

fn main() {
    let x = vec![0u8; 500];
    //~^ ERROR: useless use of `vec!`
    x.contains(&1);
    let y = vec![0u8; 501];
    y.contains(&1);
}
