#![warn(duration_subsec)]

#![feature(duration_extras)]

use std::time::Duration;

fn main() {
    let dur = Duration::new(5, 0);

    let bad_micros = dur.subsec_nanos() / 1_000;
    let good_micros = dur.subsec_micros();
    assert_eq!(bad_micros, good_micros);

    let bad_millis = dur.subsec_nanos() / 1_000_000;
    let good_millis = dur.subsec_millis();
    assert_eq!(bad_millis, good_millis);

    // Handle refs
    let _ = (&dur).subsec_nanos() / 1_000;

    // Other literals aren't linted
    let _ = dur.subsec_nanos() / 699;
}
