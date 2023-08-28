#![allow(unused_imports, clippy::needless_return)]
#![warn(clippy::filter_map_identity)]

fn main() {
    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(|x| x);
    //~^ ERROR: use of `filter_map` with an identity function
    //~| NOTE: `-D clippy::filter-map-identity` implied by `-D warnings`

    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(std::convert::identity);
    //~^ ERROR: use of `filter_map` with an identity function

    use std::convert::identity;
    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(identity);
    //~^ ERROR: use of `filter_map` with an identity function

    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(|x| return x);
    //~^ ERROR: use of `filter_map` with an identity function
}
