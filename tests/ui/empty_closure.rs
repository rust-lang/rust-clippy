#![warn(clippy::empty_closure)]

fn main() {
    // Lint
    std::thread::spawn(|| {});
    // No lint
    vec![0, 1, 2].iter().map(|_| {}).collect::<Vec<()>>();
}
