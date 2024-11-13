use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{is_in_cfg_test, is_in_test_function};
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for test functions (functions annotated with `#[test]`) that prefixed with `test_`
    /// which is redundant.
    ///
    /// ### Why is this bad?
    /// This is redundant because the test functions are already annotated with `#[test]`.
    /// Moreover, it clutters the output of `cargo test` as test functions are expanded as
    /// `module::tests::test_name` in the output.
    ///
    /// ### Example
    /// ```no_run
    /// #[test]
    /// fn test_my_use_case() {
    ///     // test code
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// fn my_use_case() {
    ///    // test code
    /// }
    /// ```
    #[clippy::version = "1.84.0"]
    pub REDUNDANT_TEST_PREFIX,
    pedantic,
    "redundant `test_` prefix in test function name"
}

pub struct RedundantTestPrefix {
    in_integration_tests: bool,
}

impl RedundantTestPrefix {
    pub fn new(conf: &'static Conf) -> Self {
        Self {
            in_integration_tests: conf.redundant_test_prefix_in_integration_tests,
        }
    }
}

impl_lint_pass!(RedundantTestPrefix => [REDUNDANT_TEST_PREFIX]);

impl LateLintPass<'_> for RedundantTestPrefix {
    fn check_fn(
        &mut self,
        cx: &LateContext<'_>,
        fn_kind: FnKind<'_>,
        _: &FnDecl<'_>,
        body: &Body<'_>,
        span: Span,
        _: LocalDefId,
    ) {
        // Skip the lint if the function is not within a node with `#[cfg(test)]` attribute,
        // which is true for integration tests. If the lint is enabled for integration tests,
        // via configuration value, ignore this check.
        if !(self.in_integration_tests || is_in_cfg_test(cx.tcx, body.id().hir_id)) {
            return;
        }

        if is_in_test_function(cx.tcx, body.id().hir_id) && has_redundant_test_prefix(fn_kind) {
            span_lint_and_help(
                cx,
                REDUNDANT_TEST_PREFIX,
                span,
                "redundant `test_` prefix in test function name",
                None,
                "consider removing the `test_` prefix",
            );
        }
    }
}

/// Checks if the function has redundant `test_` prefix in its name.
fn has_redundant_test_prefix(fn_kind: FnKind<'_>) -> bool {
    matches!(fn_kind, FnKind::ItemFn(ident, ..) if ident.name.as_str().starts_with("test_"))
}
