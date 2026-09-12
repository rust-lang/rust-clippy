#![warn(clippy::test_without_assertions)]

mod main {
    use std::{assert_matches, debug_assert_matches};

    #[test]
    fn empty() {}
    //~^ test_without_assertions

    #[test]
    fn empty_with_comment() {
        // This is still an empty body.
    }
    //~^^^ test_without_assertions

    fn ordinary_empty_function() {}

    #[test]
    fn no_failure_site() {
        let _ = 1 + 1;
    }
    //~^^^ test_without_assertions

    #[expect(clippy::unused_unit)]
    #[test]
    fn only_returns_unit() {
        ()
    }
    //~^^^ test_without_assertions

    #[test]
    #[should_panic]
    fn empty_should_panic() {}
    //~^ test_without_assertions

    #[test]
    #[should_panic(expected = "intentional")]
    fn should_panic_with_expected_message() {
        let _ = 1 + 1;
    }

    #[test]
    #[should_panic]
    fn should_panic_without_visible_panic() {
        let _ = 1 + 1;
    }

    #[test]
    fn calls_helper() {
        // False negative: calls are treated as potential failure sites without inspecting their bodies.
        helper();
    }

    #[test]
    fn explicit_panic() {
        panic!("intentional")
    }

    #[test]
    fn assert_equal() {
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn assert_not_equal() {
        assert_ne!(1 + 1, 3);
    }

    #[test]
    fn assertion() {
        let val = 1 + 1;
        assert!(val == 2);
    }

    #[test]
    fn debug_assertion() {
        let val = 1 + 1;
        debug_assert!(val == 2);
    }

    #[test]
    fn assertion_matches() {
        let val: Result<i32, &str> = Ok(42);
        assert_matches!(val, Ok(42));
    }

    #[test]
    fn debug_assertion_matches() {
        let val = Some(42);
        debug_assert_matches!(val, Some(n) if n > 0);
    }

    fn helper() {}
}
