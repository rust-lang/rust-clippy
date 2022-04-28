#![warn(clippy::manual_non_exhaustive)]
#![allow(unused)]

enum E {
    A,
    B,
    #[doc(hidden)]
    _C,
}

// user forgot to remove the marker
#[non_exhaustive]
enum Ep {
    A,
    B,
    #[doc(hidden)]
    _C,
}

// marker variant does not have doc hidden attribute, should be ignored
enum NoDocHidden {
    A,
    B,
    _C,
}

// name of variant with doc hidden does not start with underscore, should be ignored
enum NoUnderscore {
    A,
    B,
    #[doc(hidden)]
    C,
}

// variant with doc hidden is not unit, should be ignored
enum NotUnit {
    A,
    B,
    #[doc(hidden)]
    _C(bool),
}

// variant with doc hidden is the only one, should be ignored
enum OnlyMarker {
    #[doc(hidden)]
    _A,
}

// variant with multiple markers, should be ignored
enum MultipleMarkers {
    A,
    #[doc(hidden)]
    _B,
    #[doc(hidden)]
    _C,
}

// already non_exhaustive and no markers, should be ignored
#[non_exhaustive]
enum NonExhaustive {
    A,
    B,
}

// marked is used, don't lint
enum UsedHidden {
    #[doc(hidden)]
    _A,
    B,
    C,
}
fn foo(x: &mut UsedHidden) {
    if matches!(*x, UsedHidden::B) {
        *x = UsedHidden::_A;
    }
}

fn main() {}
