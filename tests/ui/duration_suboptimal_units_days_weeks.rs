#![warn(clippy::duration_suboptimal_units)]
// The duration_constructors feature enables `Duration::from_days` and `Duration::from_weeks`, so we
// should suggest them
#![feature(duration_constructors)]

use std::time::Duration;

fn main() {
    // Literals with small promoted values should not lint (issue #16532)
    let dur = Duration::from_secs(60);
    let dur = Duration::from_hours(24);

    // Literals with large promoted values should still lint
    let dur = Duration::from_hours(264);
    //~^ duration_suboptimal_units

    // Expressions should always lint
    let dur = Duration::from_nanos(13 * 7 * 24 * 60 * 60 * 1_000 * 1_000 * 1_000);
    //~^ duration_suboptimal_units
    let dur = Duration::from_hours(2 * 12);
    //~^ duration_suboptimal_units
}
