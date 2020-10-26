// run-rustfix
#![allow(dead_code)]
#![warn(clippy::manual_duration_calcs)]

use std::time::Duration;

fn main() {
    let dur = Duration::new(5, 0);

    let bad_millis_1 = dur.subsec_micros() / 1_000;
    let bad_millis_2 = dur.subsec_nanos() / 1_000_000;
    let good_millis = dur.subsec_millis();
    assert_eq!(bad_millis_1, good_millis);
    assert_eq!(bad_millis_2, good_millis);

    let bad_micros = dur.subsec_nanos() / 1_000;
    let good_micros = dur.subsec_micros();
    assert_eq!(bad_micros, good_micros);

    // Handle refs
    let _ = (&dur).subsec_nanos() / 1_000;

    // Handle constants
    const NANOS_IN_MICRO: u32 = 1_000;
    let _ = dur.subsec_nanos() / NANOS_IN_MICRO;

    // Other literals aren't linted
    let _ = dur.subsec_nanos() / 699;

    // Manual implementation
    let _nanos = diff.as_secs() * 1_000_000_000 + diff.subsec_nanos() as u64;
    let _milis = diff.as_secs() * 1_000 + diff.subsec_milis() as u64;
    let _secs_f64_1 = (diff.as_secs() as f64 * 1_000.0 + diff.subsec_milis() as f64) / 100.0;
    let _secs_f64_2 = diff.as_secs() as f64 + diff.subsec_milis() as f64 / 1_000.0;
    let _secs_f64_3 = diff.as_secs() as f64 + diff.subsec_nanos() as f64 / 1_000_000_000.0;
}
