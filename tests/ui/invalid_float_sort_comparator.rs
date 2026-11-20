#![warn(clippy::invalid_float_sort_comparator)]

use std::cmp::Ordering;

fn invalid_float_sort_comparator<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).unwrap_or(Ordering::Equal)
}

fn valid_float_sort_comparator<T: PartialOrd>(a: &T, b: &T) -> Ordering {
    a.partial_cmp(b).unwrap()
}

fn main() {
    // Basic detection: f32
    let mut floats32: [f32; 0] = [];
    floats32.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    //~^ invalid_float_sort_comparator
    floats32.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    //~^ invalid_float_sort_comparator

    // Basic detection: f64
    let mut floats64: [f64; 0] = [];
    floats64.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    //~^ invalid_float_sort_comparator
    floats64.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    //~^ invalid_float_sort_comparator

    // Named generic comparator, instantiated with floats
    floats32.sort_by(invalid_float_sort_comparator);
    //~^ invalid_float_sort_comparator
    floats64.sort_unstable_by(invalid_float_sort_comparator);
    //~^ invalid_float_sort_comparator

    // No warning: integer comparison
    let mut integers: [i32; 0] = [];
    integers.sort_by(invalid_float_sort_comparator);
    integers.sort_unstable_by(invalid_float_sort_comparator);

    // No warning: total_cmp
    floats64.sort_by(f64::total_cmp);
    floats64.sort_unstable_by(f64::total_cmp);

    // No warning: explicitly rejecting incomparable values
    floats64.sort_by(|a, b| a.partial_cmp(b).unwrap());
    floats64.sort_by(valid_float_sort_comparator);

    // No warning: valid fallback
    floats64.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));
    floats64.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Greater));

    // No warning: partial_cmp outside sorting
    let a = f64::NAN;
    let b = 1.0;
    let _ = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
}
