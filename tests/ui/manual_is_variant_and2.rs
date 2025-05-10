#![warn(clippy::manual_is_variant_and)]

fn main() {
    let _ = Some(2).map(|x| x % 2 == 0) == Some(true);
    //~^ manual_is_variant_and
    let _ = Some(2).map(|x| x % 2 == 0) != Some(true);
    //~^ manual_is_variant_and
    let _ = Ok::<usize, ()>(2).map(|x| x % 2 == 0) == Ok(true);
    //~^ manual_is_variant_and
    let _ = Ok::<usize, ()>(2).map(|x| x % 2 == 0) != Ok(true);
    //~^ manual_is_variant_and
}
