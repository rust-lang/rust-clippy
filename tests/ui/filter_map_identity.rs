#![allow(unused_imports, clippy::needless_return)]
#![warn(clippy::filter_map_identity)]

fn main() {
    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(|x| x); //~ filter_map_identity

    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(std::convert::identity);
    //~^ filter_map_identity

    use std::convert::identity;
    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(identity); //~ filter_map_identity

    let iterator = vec![Some(1), None, Some(2)].into_iter();
    let _ = iterator.filter_map(|x| return x);
    //~^ filter_map_identity
}
