// compile-flags: --crate-name=unknown_features --cfg feature="misspelled" --cfg feature="another"
#![warn(clippy::unknown_features)]

fn main() {
    #[cfg(feature = "mispelled")]
    let _ = 42;

    #[cfg(feature = "dependency/unknown")]
    let _ = 42;

    #[cfg(any(not(feature = "misspeled"), feature = "not-found"))]
    let _ = 21;

    if cfg!(feature = "nothe") {}
}
