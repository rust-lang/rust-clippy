//! This module is intended to hold most implementations related to Clippy's
//! nightly lints.

/// This determines if Clippy is being compiled for nightly or stable. Bootstrap which is
/// used to compile Clippy in the rust repo will set the `CFG_DISABLE_UNSTABLE_FEATURES`
/// environment value if Clippy is being compiled for nightly (Or another edge case see
/// [`rustc_feature::UnstableFeatures`]). In the rust-clippy repo, Clippy is being compiled
/// with cargo, therefore we can just check if a cargo environment value is set to determine
/// if this is a nightly run or not
const IS_NIGHTLY: bool =
    std::option_env!("CFG_DISABLE_UNSTABLE_FEATURES").is_some() || std::option_env!("CARGO_MANIFEST_DIR").is_some();

/// This function checks if the current run is a nightly run with Clippy's nightly lints. This is
/// destinct from rustc's as a nightly build can disable Clippy's nightly features.
///
/// See [`Session::is_nightly_build(&self)`] if you want to check if the current build is a nightly
/// build.
#[inline]
pub const fn is_nightly_run() -> bool {
    IS_NIGHTLY
}
