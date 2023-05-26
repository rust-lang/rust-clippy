#![warn(clippy::bool_comparison)]

fn main() {
    let x = true;
    let _ = x == false;
    let _ = false == x;
    let _ = x != true;
    let _ = true != x;
    let _ = x < true;
    let _ = true > x;

    let y = true;
    let _ = x < y;
    let _ = x > y;
    let _ = x > y && !x && y == false;
}
