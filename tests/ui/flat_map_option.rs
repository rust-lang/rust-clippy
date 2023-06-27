//@run-rustfix
#![warn(clippy::flat_map_option)]
#![allow(
    clippy::iter_on_single_items,
    clippy::redundant_closure,
    clippy::unnecessary_filter_map
)]

fn main() {
    // yay
    let c = |x| Some(x);
    let _ = [1].iter().flat_map(c);
    let _ = [1].iter().flat_map(Some);

    // nay
    let _ = [1].iter().flat_map(|_| &Some(1));
}
