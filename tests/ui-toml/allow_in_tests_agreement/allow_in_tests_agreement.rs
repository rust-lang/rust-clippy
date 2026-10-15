//@compile-flags: --test
//@no-rustfix
//! Both late pass (`dbg_macro`) and early pass (`needless_raw_strings`)
//! lints should be suppressable by `allow-in-tests`.
#![warn(clippy::dbg_macro, clippy::needless_raw_strings)]

fn main() {}

// Not test code: both fire.
fn plain() {
    let _ = r"no escapes";
    //~^ needless_raw_strings
    let _ = dbg!(0);
    //~^ dbg_macro
}

#[test]
fn in_test_fn() {
    // `is_in_test_function`
    let _ = r"no escapes";
    let _ = dbg!(0);
}

#[cfg(test)]
mod in_cfg_test {
    // Not a `#[test]` function, so this is `is_in_cfg_test` rather than `is_in_test_function`.
    fn helper() {
        let _ = r"no escapes";
        let _ = dbg!(0);
    }

    #[test]
    fn uses_helper() {
        helper();
    }
}
