// Tracking issue: #3913
#![deny(clippy::explicit_iter_loop)]

fn main() {
    let vec = vec![1];
    for _v in vec.iter() {} //~ ERROR change to `&vec`
}
