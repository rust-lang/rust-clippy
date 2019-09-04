#![warn(clippy::map_flatten_filtermap)]

fn main() {
    let v = [10, 20, 30];

    let filter1 = v
        .iter()
        .map(|x| if *x > 10 { Some(x) } else { None })
        .flatten()
        .collect::<Vec<_>>();

    let filtered2 = v
        .iter()
        .filter_map(|x| if *x > 10 { Some(x) } else { None })
        .collect::<Vec<_>>();

    assert_eq!(filtered1, filtered2);
}