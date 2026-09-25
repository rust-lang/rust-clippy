#![warn(clippy::useless_vec)]

fn main() {
    let x = vec![0u8; 500];
    //~^ useless_vec
    x.contains(&1);
    let y = vec![0u8; 501];
    y.contains(&1);
}
