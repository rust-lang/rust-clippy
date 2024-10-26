#![allow(unused)]
#![allow(clippy::tests_outside_test_module)]
#![allow(clippy::unnecessary_literal_unwrap)]
#![warn(clippy::test_without_fail_case)]

struct DummyStruct;

impl DummyStruct {
    fn panic_in_impl(self) {
        panic!("a")
    }

    fn assert_in_impl(self, a: bool) {
        assert!(a)
    }

    fn unwrap_in_impl(self, a: Option<i32>) {
        let _ = a.unwrap();
    }
}

#[test]
fn test_without_fail() {
    let x = 5;
    let y = x + 2;
    println!("y: {}", y);
}

// Should not lint
#[test]
fn impl_panic() {
    let dummy_struct = DummyStruct;
    dummy_struct.panic_in_impl();
}

// Should not lint
#[test]
fn impl_assert() {
    let dummy_struct = DummyStruct;
    dummy_struct.assert_in_impl(false);
}

// Should not lint
#[test]
fn impl_unwrap() {
    let dummy_struct = DummyStruct;
    let a = Some(10i32);
    dummy_struct.unwrap_in_impl(a);
}

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

// Should not lint
#[test]
fn test_implicit_unwrap() {
    implicit_unwrap();
}

// Should not lint
#[test]
fn test_implicit_assert() {
    implicit_assert();
}

// Should lint with default config.
#[test]
fn test_slice_index() {
    let a = [1, 2, 3, 4, 5, 6];
    // indexing into slice, this can fail but by default check for this is disabled.
    let b = a[0];
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
