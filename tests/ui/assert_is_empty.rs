#![warn(clippy::assert_is_empty)]
#![allow(clippy::len_zero, clippy::comparison_to_empty)]

fn main() {
    // Test with Vec
    let v: Vec<i32> = vec![];
    assert!(v.is_empty());
    //~^ assert_is_empty
    assert!(!v.is_empty());
    //~^ assert_is_empty

    // Test with String
    let s = String::new();
    assert!(s.is_empty());
    //~^ assert_is_empty
    assert!(!s.is_empty());
    //~^ assert_is_empty

    // Should not lint: custom message
    assert!(v.is_empty(), "vec is not empty");
    assert!(!v.is_empty(), "vec is empty");

    // Should not lint: assert_ne!/assert_eq! already fine
    assert_eq!(v, []);
    assert_ne!(v, []);

    // Should not lint: not is_empty
    assert!(v.len() == 0);
}
