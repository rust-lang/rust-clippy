use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{is_in_cfg_test, is_in_test_function};
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::def_id::LocalDefId;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Triggers when a function annotated with the `#[test]` macro begins with `test_`
    /// ### Why is this bad?
    /// This is redundant, since the annotations already denotes the function a test.
    /// Tests are executed using `cargo test `module::test-name-here::`.
    /// It's clear that a test is a test, without prefixing `test_` to the test name.
    /// ### Example
    /// ```no_run
    /// #[test]
    /// fn my_test_case() {
    ///     // test logic here...
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// #[test]
    /// fn my_test_case() {
    ///     // test logic here...
    /// }
    /// ```
    #[clippy::version = "1.79.0"]
    pub REDUNDANT_TEST_PREFIX,
    pedantic,
    "A test name is prefixed with `test`"
}

declare_lint_pass!(RedundantTestPrefix => [REDUNDANT_TEST_PREFIX]);

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
        if is_prefixed_with_test_fn(fn_kind)
            && is_in_test_function(cx.tcx, body.id().hir_id)
            && is_in_cfg_test(cx.tcx, body.id().hir_id)
        {
            span_lint_and_help(
                cx,
                REDUNDANT_TEST_PREFIX,
                span,
                "this test is prefixed redundantly with `test`",
                None,
                "consider removing the redundant `test` prefix from the test name",
            );
        }
    }
}

fn is_prefixed_with_test_fn(fn_kind: FnKind<'_>) -> bool {
    match fn_kind {
        FnKind::ItemFn(ident, ..) => {
            // check if `fn` name is prefixed with `test_`
            ident.name.as_str().starts_with("test_")
        },
        // ignore closures
        _ => false,
    }
}
