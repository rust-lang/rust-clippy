#![warn(clippy::uninformative_asserts)]
#![allow(clippy::blacklisted_name)]

fn main() {
    let foo = 0u8;
    let a = 0u8;
    let b = 0u8;
    // lint
    {
        assert!(some_condition(foo));
        assert!(some_condition(foo),);
        debug_assert!(some_condition(foo));
        assert_eq!(a, bar(b));
        assert_eq!(a, bar(b),);
        assert_ne!(a, bar(b));
        debug_assert_eq!(a, bar(b));
        debug_assert_ne!(a, bar(b));
    }

    // ok
    {
        assert!(some_condition(foo), "_");
        assert!(some_condition(foo), "{} {}", 1, 2);
        debug_assert!(some_condition(foo), "_");
        assert_eq!(a, bar(b), "_");
        assert_eq!(a, bar(b), "{} {}", 1, 2);
        assert_ne!(a, bar(b), "_");
        debug_assert_eq!(a, bar(b), "_");
        debug_assert_ne!(a, bar(b), "_");
    }
}

#[test]
fn test_something() {
    assert_eq!(bar(0), 0);
}

fn some_condition(_x: u8) -> bool {
    true
}
fn bar(x: u8) -> u8 {
    x
}
