//@no-rustfix
#![warn(clippy::manual_filter)]

// the suggestion is missing the necessary adjustments
fn issue16031() -> Option<&'static i32> {
    match Some(&&0) {
        //~^ manual_filter
        None => None,
        Some(x) => {
            if **x > 0 {
                None
            } else {
                Some(x)
            }
        },
    }
}
