#![warn(clippy::bool_assert_inverted)]

use std::ops::Not;

macro_rules! a {
    () => {
        true
    };
}
macro_rules! b {
    () => {
        true
    };
}

#[derive(Debug, Clone, Copy)]
struct ImplNotTraitWithBool;

impl PartialEq<bool> for ImplNotTraitWithBool {
    fn eq(&self, other: &bool) -> bool {
        false
    }
}

impl Not for ImplNotTraitWithBool {
    type Output = bool;

    fn not(self) -> Self::Output {
        true
    }
}

fn main() {
    let a = ImplNotTraitWithBool;

    assert!(!"a".is_empty());
    assert!("".is_empty());
    assert!(!a);
    assert!(a);

    debug_assert!(!"a".is_empty());
    debug_assert!("".is_empty());
    debug_assert!(!a);
    debug_assert!(a);

    assert!(!"a".is_empty(), "tadam {}", false);
    assert!("".is_empty(), "tadam {}", false);
    assert!(!a, "tadam {}", false);
    assert!(a, "tadam {}", false);

    debug_assert!(!"a".is_empty(), "tadam {}", false);
    debug_assert!("".is_empty(), "tadam {}", false);
    debug_assert!(!a, "tadam {}", false);
    debug_assert!(a, "tadam {}", false);
}
