use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_in_cfg_test, is_in_test_function};
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::def_id::LocalDefId;
use rustc_span::source_map::SourceMap;
use rustc_span::{FileName, Span};
use std::path::PathBuf;

declare_clippy_lint! {
    /// ### What it does
    /// Triggers when a testing function (marked with the `#[test]` attribute) isn't inside a testing module
    /// (marked with `#[cfg(test)]`).
    ///
    /// ### Why restrict this?
    /// The idiomatic (and more performant) way of writing tests is inside a testing module (flagged with `#[cfg(test)]`),
    /// having test functions outside of this module is confusing and may lead to them being "hidden".
    /// This does not apply to integration tests though, and this lint will ignore those.
    ///
    /// ### Example
    /// ```no_run
    /// #[test]
    /// fn my_cool_test() {
    ///     // [...]
    /// }
    ///
    /// #[cfg(test)]
    /// mod tests {
    ///     // [...]
    /// }
    ///
    /// ```
    /// Use instead:
    /// ```no_run
    /// #[cfg(test)]
    /// mod tests {
    ///     #[test]
    ///     fn my_cool_test() {
    ///         // [...]
    ///     }
    /// }
    /// ```
    #[clippy::version = "1.70.0"]
    pub TESTS_OUTSIDE_TEST_MODULE,
    restriction,
    "A test function is outside the testing module."
}

declare_lint_pass!(TestsOutsideTestModule => [TESTS_OUTSIDE_TEST_MODULE]);

impl LateLintPass<'_> for TestsOutsideTestModule {
    fn check_fn(
        &mut self,
        cx: &LateContext<'_>,
        kind: FnKind<'_>,
        _: &FnDecl<'_>,
        body: &Body<'_>,
        sp: Span,
        _: LocalDefId,
    ) {
        if !matches!(kind, FnKind::Closure)
            && is_in_test_function(cx.tcx, body.id().hir_id)
            && !is_integration_test(cx.tcx.sess.source_map(), sp)
            && !is_in_cfg_test(cx.tcx, body.id().hir_id)
        {
            #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
            span_lint_and_then(
                cx,
                TESTS_OUTSIDE_TEST_MODULE,
                sp,
                "this function marked with #[test] is outside a #[cfg(test)] module",
                |diag| {
                    diag.note("move it to a testing module marked with #[cfg(test)]");
                },
            );
        }
    }
}

fn is_integration_test(sm: &SourceMap, sp: Span) -> bool {
    // This part is from https://github.com/rust-lang/rust/blob/a91f7d72f12efcc00ecf71591f066c534d45ddf7/compiler/rustc_expand/src/expand.rs#L402-L409 (fn expand_crate)
    // Extract crate base path from the filename path.
    let file_path = match sm.span_to_filename(sp) {
        FileName::Real(name) => name
            .into_local_path()
            .expect("attempting to resolve a file path in an external file"),
        other => PathBuf::from(other.prefer_local().to_string()),
    };

    // Root path contains the topmost sources directory of the crate.
    // I.e., for `project` with sources in `src` and tests in `tests` folders
    // (no matter how many nested folders lie inside),
    // there will be two different root paths: `/project/src` and `/project/tests`.
    let root_path = file_path.parent().unwrap_or(&file_path).to_owned();

    // The next part matches logic in https://github.com/rust-lang/rust/blob/a91f7d72f12efcc00ecf71591f066c534d45ddf7/compiler/rustc_builtin_macros/src/test.rs#L526 (fn test_type)
    // Integration tests are under /tests directory in the crate root_path we determined above.
    root_path.ends_with("tests")
}
