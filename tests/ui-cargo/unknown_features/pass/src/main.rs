// compile-flags: --crate-name=unknown_features --cfg feature="fancy" --cfg feature="another"
// compile-flags: --cfg feature="serde/derive"
#![warn(clippy::unknown_features)]

fn main() {
    #[cfg(feature = "fancy")]
    let _ = 42;

    #[cfg(feature = "serde/derive")]
    let _ = 42;

    #[cfg(any(not(feature = "fancy"), feature = "another"))]
    let _ = 21;

    if cfg!(feature = "fancy") {}
}
