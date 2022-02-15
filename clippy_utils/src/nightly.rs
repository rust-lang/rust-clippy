//! This module is intended to hold most implementations related to Clippy's
//! nightly lints.

use std::lazy::SyncOnceCell;

use rustc_data_structures::stable_set::FxHashSet;
use rustc_lint::{EarlyContext, LateContext, Level, Lint, LintId};
use rustc_middle::lint::{LevelAndSource, LintLevelSource};
use rustc_session::Session;

static IS_NIGHTLY_RUN: SyncOnceCell<bool> = SyncOnceCell::new();
static NIGHTLY_LINTS: SyncOnceCell<FxHashSet<LintId>> = SyncOnceCell::new();

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
/// See [`Session::is_nightly_build(&self)`] if you want to check if the current build is a nightly
/// build.
#[inline]
pub fn is_nightly_run() -> bool {
    *IS_NIGHTLY_RUN.get().unwrap_or(&false)
}

/// This function takes a list of all nightly lints that will be surpressed before
/// the emission if nightly lints are disabled.
///
/// It's only allowed to call this once. This is done by [`clippy_lints::lib`]
#[doc(hidden)]
pub fn set_nightly_lints<const N: usize>(lints: [LintId; N]) {
    // The from trait for HashMaps is only implemented for the normal hasher. Here we have to add each
    // item individually
    let mut nightly_lints = FxHashSet::default();
    lints.iter().copied().for_each(|lint| {
        nightly_lints.insert(lint);
    });
    NIGHTLY_LINTS
        .set(nightly_lints)
        .expect("`NIGHTLY_LINTS` should only be set once.");
}

/// Returns true if the lint is a registered nightly lint. Note that a lint will still be a
/// registered nightly lint if nightly lints are enabled as usual.
///
/// Please use [`is_nightly_run`] to determine if Clippy's nightly features
/// should be enabled.
#[inline]
pub fn is_nightly_lint(lint: &'static Lint) -> bool {
    NIGHTLY_LINTS
        .get()
        .map_or(false, |lints| lints.contains(&LintId::of(lint)))
}

/// This function checks if the given lint is a nightly lint and should be suppressed in the current
/// context.
#[inline]
pub fn suppress_lint<T: LintLevelProvider>(cx: &T, lint: &'static Lint) -> bool {
    if !is_nightly_run() && is_nightly_lint(lint) {
        let (_, level_src) = cx.get_lint_level(lint);
        if level_src == LintLevelSource::Default
            || level_src == LintLevelSource::CommandLine(sym!(warnings), Level::Deny)
        {
            return true;
        }
    }

    false
}

/// This trait is used to retrieve the lint level for the lint based on the
/// current linting context.
pub trait LintLevelProvider {
    fn get_lint_level(&self, lint: &'static Lint) -> LevelAndSource;
}

impl LintLevelProvider for LateContext<'_> {
    fn get_lint_level(&self, lint: &'static Lint) -> LevelAndSource {
        self.tcx.lint_level_at_node(lint, self.last_node_with_lint_attrs)
    }
}

impl LintLevelProvider for EarlyContext<'_> {
    fn get_lint_level(&self, lint: &'static Lint) -> LevelAndSource {
        self.builder.lint_level(lint)
    }
}
