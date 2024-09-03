#![warn(clippy::unused_any_all)]

fn main() {
    (0..1).any(|_| false);
    Iterator::any(&mut (0..1), |_| false);
    let _ = (0..1).any(|_| false);
    let _ = Iterator::any(&mut (0..1), |_| false);
}
