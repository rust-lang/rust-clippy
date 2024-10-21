#![allow(unused)]
#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::unnecessary_literal_unwrap)]
#![warn(clippy::test_without_fail_case)]

// Should lint
#[test]
fn test_without_fail() {
    // This test cannot fail.
    let x = 5;
    let y = x + 2;
    println!("y: {}", y);
}
//~^ ERROR: this function marked with #[test] does not have a way to fail.
//~^ NOTE: Ensure that something is being tested and asserted by this test.

// Should not lint
#[test]
fn test_with_fail() {
    // This test can fail.
    assert_eq!(1 + 1, 2);
}

// Should not lint
#[test]
fn test_implicit_panic() {
    implicit_panic()
}

fn implicit_panic() {
    panic!("this is an implicit panic");
}

fn implicit_unwrap() {
    let val: Option<u32> = None;
    let _ = val.unwrap();
}

fn implicit_assert() {
    assert_eq!(1, 2)
}

fn main() {
    // Non-test code
}
