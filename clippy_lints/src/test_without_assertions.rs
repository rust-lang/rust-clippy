use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint;
use clippy_utils::is_test_function;
use clippy_utils::visitors::for_each_expr_without_closures;
use rustc_hir::intravisit::FnKind;
use rustc_hir::{Body, ExprKind, FnDecl, find_attr};
use rustc_lint::{LateContext, LateLintPass, declare_lint_pass};
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for test functions without assertions or other potential failure sites.
    ///
    /// ### Why is this bad?
    /// Such tests may pass without verifying any behavior. They can represent unfinished work or
    /// forgotten stubs, giving a false sense of coverage.
    ///
    /// ### Example
    /// ```no_run
    /// #[test]
    /// fn adds_numbers() {
    ///     let _ = 1 + 1;
    /// }
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// #[test]
    /// fn adds_numbers() {
    ///     assert_eq!(1 + 1, 2);
    /// }
    /// ```
    ///
    /// ### Known problems
    ///
    /// Calls and method calls are treated as potential failure sites without inspecting their
    /// bodies. Therefore, tests that only call helpers which cannot fail are not linted.
    #[clippy::version = "1.100.0"]
    pub TEST_WITHOUT_ASSERTIONS,
    suspicious,
    "test function without assertions or other potential failure sites"
}

declare_lint_pass!(TestWithoutAssertions => [TEST_WITHOUT_ASSERTIONS]);

impl LateLintPass<'_> for TestWithoutAssertions {
    fn check_fn(
        &mut self,
        cx: &LateContext<'_>,
        kind: FnKind<'_>,
        _: &FnDecl<'_>,
        body: &Body<'_>,
        span: Span,
        fn_def_id: LocalDefId,
    ) {
        if matches!(kind, FnKind::ItemFn(..))
            && let ExprKind::Block(block, _) = body.value.kind
            && is_test_function(cx.tcx, fn_def_id)
        {
            let is_empty = block.stmts.is_empty() && block.expr.is_none();
            let is_should_panic = find_attr!(cx.tcx, fn_def_id, ShouldPanic { .. });

            if is_empty || (!is_should_panic && !has_potential_failure_site(body)) {
                span_lint(
                    cx,
                    TEST_WITHOUT_ASSERTIONS,
                    span,
                    "test function without assertions or other potential failure sites",
                );
            }
        }
    }
}

fn has_potential_failure_site(body: &Body<'_>) -> bool {
    for_each_expr_without_closures(body.value, |expr| {
        if matches!(expr.kind, ExprKind::Call(..) | ExprKind::MethodCall(..)) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_some()
}
