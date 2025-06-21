#![warn(clippy::manual_exhaustive_patterns)]

fn main() {
    matches!(Some(6), Some(_) | None);
    //~^ manual_exhaustive_patterns
    matches!(Some(6), None | Some(_));
    //~^ manual_exhaustive_patterns
    matches!(Ok::<i32, i32>(6), Ok(_) | Err(_));
    //~^ manual_exhaustive_patterns
    matches!(true, true | false);
    //~^ manual_exhaustive_patterns
}
