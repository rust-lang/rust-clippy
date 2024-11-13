#![warn(clippy::BUFREADER_STDIN)]
use std::io::{self, BufReader};

fn main() {
    let a = io::stdin();
    let reader = BufReader::new(a);

    let b = io::stdin().lock();
    let reader = BufReader::new(b);
}
