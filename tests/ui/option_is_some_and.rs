#![allow(unused)]
#![warn(clippy::option_is_some_and)]

fn main() {
    let x: Option<int> = Some(7);
    x.map(|val| val > 5).unwrap_or(false); //must be caught

    let y: Option<int> = Some(9);
    y.map(|val| val > 5).unwrap_or(true); //should not be caught

    let z: Option<bool> = Some(true);
    z.map(|val1| val1).map(|val2| !val2).unwrap_or(false).then(|| 0); // must be caught
}
