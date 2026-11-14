#![warn(clippy::needless_collect)]

fn main() {
    let _ = [1, 2, 3].into_iter().collect::<Vec<_>>().is_empty();
    //~^ needless_collect
}
