#![warn(clippy::ifs_as_logical_ops)]

fn main() {
    // test code goes here
}

fn john(b1: bool, b2: bool) -> bool {
    if b1 { b2 } else { false }
    //~^ ifs_as_logical_ops
}
