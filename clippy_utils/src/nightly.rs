//! This module is intended to hold most implementations related to Clippy's
//! nightly lints.

use std::lazy::SyncOnceCell;

use rustc_session::Session;

static IS_NIGHTLY_RUN: SyncOnceCell<bool> = SyncOnceCell::new();

/// This function is used to determine if nightly lints should be enabled or disabled
/// in this Clippy run.
///
/// It's only allowed to call this once. This is done by [`clippy_lints::lib`]
pub fn eval_is_nightly_run(sess: &Session) {
    // This allows users to disable nightly lints on nightly
    let disable_nightly = std::env::var("CLIPPY_NIGHTLY").map(|s| s == "0").unwrap_or(false);
    // This allows users to enable nightly lints on stable
    let enable_nightly = std::env::var("CLIPPY_NIGHTLY").map(|s| s == "1").unwrap_or(false);

    let is_nightly_run = enable_nightly || (sess.is_nightly_build() && !disable_nightly);

    IS_NIGHTLY_RUN
        .set(is_nightly_run)
        .expect("`ENABLE_NIGHTLY_LINTS` should only be set once.");
}

/// This function checks if the current run is a nightly run with Clippy's nightly lints. This is
/// destinct from rustc's as a nightly build can disable Clippy's nightly features.
/// 
/// See [`Session::is_nightly_build(&self)`] if you want to check if the current build is a nightly build.
#[inline]
pub fn is_nightly_run() -> bool {
    *IS_NIGHTLY_RUN.get().unwrap_or(&false)
}
