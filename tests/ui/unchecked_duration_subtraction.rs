#![warn(clippy::unchecked_duration_subtraction)]

use std::time::{Duration, Instant};

fn main() {
    let _first = Instant::now();
    let second = Duration::from_secs(3);

    let _ = _first - second;
    //~^ ERROR: unchecked subtraction of a 'Duration' from an 'Instant'
    //~| NOTE: `-D clippy::unchecked-duration-subtraction` implied by `-D warnings`

    let _ = Instant::now() - Duration::from_secs(5);
    //~^ ERROR: unchecked subtraction of a 'Duration' from an 'Instant'

    let _ = _first - Duration::from_secs(5);
    //~^ ERROR: unchecked subtraction of a 'Duration' from an 'Instant'

    let _ = Instant::now() - second;
    //~^ ERROR: unchecked subtraction of a 'Duration' from an 'Instant'
}
