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
    let _nanos = dur.as_secs() * 1_000_000_000 + dur.subsec_nanos() as u64;
    let _milis = dur.as_secs() * 1_000 + dur.subsec_millis() as u64;
    let _secs_f64_1 = (dur.as_secs() as f64 * 1_000.0 + dur.subsec_millis() as f64) / 1000.0;
    let _secs_f64_2 = dur.as_secs() as f64 + dur.subsec_millis() as f64 / 1_000.0;
    let _secs_f64_3 = dur.as_secs() as f64 + dur.subsec_nanos() as f64 / 1_000_000_000.0;

    let _secs_f32_2 = dur.as_secs() as f32 + dur.subsec_millis() as f32 / 1_000.0;
    let _secs_f32_3 = dur.as_secs() as f32 + dur.subsec_nanos() as f32 / 1_000_000_000.0;
}
