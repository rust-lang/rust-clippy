// Tracking issue: #3913
#![deny(clippy::into_iter_on_array)]

fn main() {
    for _ in [1, 2, 3].into_iter() {} //~ ERROR equivalent to .iter()
}
