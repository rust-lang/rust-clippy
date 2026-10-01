//@compile-flags: --test
//@no-rustfix
#![warn(clippy::dbg_macro, clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]
#![allow(clippy::no_effect, clippy::unnecessary_operation)]

fn main() {}

// Outside of test code the listed lints still fire.
fn not_a_test(x: Option<u32>, s: &[u32]) -> u32 {
    dbg!(x);
    //~^ dbg_macro
    s[0];
    //~^ indexing_slicing
    x.unwrap()
    //~^ unwrap_used
}

#[test]
fn in_test_fn() {
    let x: Option<u32> = "1".parse().ok();
    let s: &[u32] = &[1];
    dbg!(x);
    s[0];
    x.unwrap();
    // `panic` isn't listed in `allow-in-tests`, so it is still linted.
    panic!("boom");
    //~^ panic
}

#[cfg(test)]
mod tests {
    // Not a `#[test]` function, but inside a `#[cfg(test)]` module.
    fn helper(x: Option<u32>, s: &[u32]) -> u32 {
        dbg!(x);
        s[0];
        x.unwrap()
    }

    #[test]
    fn uses_helper() {
        helper("1".parse().ok(), &[1]);
    }
}
