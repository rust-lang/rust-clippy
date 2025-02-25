#![warn(clippy::unchecked_duration_subtraction)]

use std::time::{Duration, Instant};

fn main() {
    let instant = Instant::now();
    let duration = Duration::from_secs(3);
    let duration2 = Duration::from_secs(1);

    let _ = instant - duration;
    //~^ unchecked_duration_subtraction

    let _ = Instant::now() - Duration::from_secs(5);
    //~^ unchecked_duration_subtraction

    let _ = instant - Duration::from_secs(5);
    //~^ unchecked_duration_subtraction

    let _ = Instant::now() - duration;
    //~^ unchecked_duration_subtraction

    let _ = Duration::from_secs(1) - duration;
    //~^ unchecked_duration_subtraction

    let _ = duration2 - duration;
    //~^ unchecked_duration_subtraction

    let _ = Instant::now().elapsed() - duration;
    //~^ unchecked_duration_subtraction
}
