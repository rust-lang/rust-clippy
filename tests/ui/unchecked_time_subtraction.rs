#![warn(clippy::unchecked_duration_subtraction)]

use std::time::{Duration, Instant};

fn main() {
    let instant = Instant::now();
    let duration = Duration::from_secs(3);
    let duration2 = Duration::from_secs(1);

    let _ = instant - duration;

    let _ = Instant::now() - Duration::from_secs(5);

    let _ = instant - Duration::from_secs(5);

    let _ = Instant::now() - duration;

    let _ = Duration::from_secs(1) - duration;

    let _ = duration2 - duration;

    let _ = Instant::now().elapsed() - duration;
}
