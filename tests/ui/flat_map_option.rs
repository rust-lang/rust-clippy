#![warn(clippy::flat_map_option)]
#![allow(clippy::redundant_closure, clippy::unnecessary_filter_map)]

fn main() {
    // yay
    let c = |x| Some(x);
    let _ = [1].iter().flat_map(c);
    //~^ ERROR: used `flat_map` where `filter_map` could be used instead
    //~| NOTE: `-D clippy::flat-map-option` implied by `-D warnings`
    let _ = [1].iter().flat_map(Some);
    //~^ ERROR: used `flat_map` where `filter_map` could be used instead

    // nay
    let _ = [1].iter().flat_map(|_| &Some(1));
}
