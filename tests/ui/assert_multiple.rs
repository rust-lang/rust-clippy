#![warn(clippy::assert_multiple)]

fn main() {
    let expected_name = "name";
    let actual_name = "name";
    let expected_count = 1;
    let actual_count = 1;

    assert!(actual_name == expected_name && actual_count == expected_count);
    //~^ assert_multiple
    debug_assert!(actual_name != expected_name && actual_count != expected_count);
    //~^ assert_multiple

    // Do not lint comparisons whose values cannot be shown by `assert_eq!`.
    #[derive(PartialEq)]
    struct NotDebug;
    let left_one = NotDebug;
    let right_one = NotDebug;
    let left_two = NotDebug;
    let right_two = NotDebug;
    assert!(left_one == right_one && left_two == right_two);

    // Keep custom messages intact rather than duplicating them.
    assert!(
        actual_name == expected_name && actual_count == expected_count,
        "context"
    );
}
