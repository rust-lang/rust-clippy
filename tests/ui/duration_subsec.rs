#![allow(dead_code, clippy::needless_borrow)]
#![warn(clippy::duration_subsec)]

use std::time::Duration;

fn main() {
    let dur = Duration::new(5, 0);

    let bad_millis_1 = dur.subsec_micros() / 1_000;
    //~^ ERROR: calling `subsec_millis()` is more concise than this calculation
    //~| NOTE: `-D clippy::duration-subsec` implied by `-D warnings`
    let bad_millis_2 = dur.subsec_nanos() / 1_000_000;
    //~^ ERROR: calling `subsec_millis()` is more concise than this calculation
    let good_millis = dur.subsec_millis();
    assert_eq!(bad_millis_1, good_millis);
    assert_eq!(bad_millis_2, good_millis);

    let bad_micros = dur.subsec_nanos() / 1_000;
    //~^ ERROR: calling `subsec_micros()` is more concise than this calculation
    let good_micros = dur.subsec_micros();
    assert_eq!(bad_micros, good_micros);

    // Handle refs
    let _ = (&dur).subsec_nanos() / 1_000;
    //~^ ERROR: calling `subsec_micros()` is more concise than this calculation

    // Handle constants
    const NANOS_IN_MICRO: u32 = 1_000;
    let _ = dur.subsec_nanos() / NANOS_IN_MICRO;
    //~^ ERROR: calling `subsec_micros()` is more concise than this calculation

    // Other literals aren't linted
    let _ = dur.subsec_nanos() / 699;
}
