#![warn(clippy::duration_to_float_precision_loss)]

use std::time::Duration;

const DURATION: Duration = Duration::from_nanos(1_234_567_890);

fn main() {
    let float_secs = DURATION.as_secs() as f64;

    macro_rules! m {
        ($d:expr) => {{
            let float_secs = $d.as_secs() as f64;
        }};
    }

    m!(DURATION);

    let float_secs = DURATION.as_secs() as f32;

    let float_secs = DURATION.as_millis() as f64 / 1_000.0;

    let float_secs = DURATION.as_micros() as f64 / 1_000_000.0;

    let float_secs = DURATION.as_micros() as f64 * 1e-6;

    let float_secs = DURATION.as_nanos() as f64 / 1_000_000_000.0;

    let float_millis = DURATION.as_millis() as f64;

    let float_micros = DURATION.as_micros() as f64;

    // should not trigger on these conversion, since they are already the max precision
    let float_nanos = DURATION.as_nanos() as f64;

    let float_micros = DURATION.as_nanos() as f64 / 1_000.0;

    let float_millis = DURATION.as_nanos() as f64 / 1_000_000.0;

    {
        let dur = &&DURATION;
        let float_millis = dur.as_millis() as f64 / 1_000.0;
    }
}

#[clippy::msrv = "1.37"]
fn msrv_1_37() {
    let float_secs = DURATION.as_secs() as f64; // should not trigger lint because MSRV is below
}

#[clippy::msrv = "1.38"]
fn msrv_1_38() {
    let float_secs = DURATION.as_secs() as f64; // should trigger lint because MSRV is satisfied
}
