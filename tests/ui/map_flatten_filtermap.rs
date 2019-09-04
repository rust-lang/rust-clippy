#![warn(clippy::map_flatten_filtermap)]

fn main() {
    let v = [10, 20, 30];

    let filter1 = v
        .iter()
        .map(|x| if *x > 10 { Some(x) } else { None })
        .flatten()
        .collect::<Vec<_>>();
}