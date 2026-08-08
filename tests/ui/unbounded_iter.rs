#![warn(clippy::unbounded_iter)]

fn main() {
    (0..).all(|x| x > 0);
    //~^ unbounded_iter
}
