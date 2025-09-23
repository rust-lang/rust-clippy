#![warn(clippy::filter_some)]

fn main() {
    let _ = Some(0).filter(|_| false);
    //~^ filter_some
    let _ = Some(0).filter(|_| 1 == 0);
    //~^ filter_some
}
