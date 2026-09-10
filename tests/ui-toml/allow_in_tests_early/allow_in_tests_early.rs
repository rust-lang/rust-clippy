//@compile-flags: --test
//@no-rustfix
//! `allow-in-tests` applies to early lint passes too, even though they run before the HIR that
//! late passes use to recognize test code.
#![warn(
    clippy::needless_raw_strings,
    clippy::single_char_lifetime_names,
    clippy::unseparated_literal_suffix
)]
#![allow(clippy::needless_lifetimes)]

fn main() {}

// Outside of test code the listed lints still fire.
fn not_a_test<'a>(x: &'a u32) -> &'a u32 {
    //~^ single_char_lifetime_names
    let _ = r"no escapes here";
    //~^ needless_raw_strings
    x
}

#[test]
fn in_test_fn() {
    let _ = r"no escapes here";
    // `unseparated_literal_suffix` isn't listed, so it is still linted.
    let _ = 123i32;
    //~^ unseparated_literal_suffix
}

#[cfg(test)]
mod tests {
    // Not a `#[test]` function, but inside a `#[cfg(test)]` module.
    fn helper<'a>(x: &'a u32) -> &'a u32 {
        let _ = r"no escapes here";
        x
    }

    #[test]
    fn uses_helper() {
        helper(&1);
    }
}
