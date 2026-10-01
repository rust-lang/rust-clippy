use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use clippy_utils::source::{snippet, snippet_indent};
use clippy_utils::sym;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks assertions that combine two equality comparisons with `&&`.
    ///
    /// ### Why is this bad?
    /// A combined assertion only reports that the whole condition failed. Separate assertions
    /// identify which comparison failed and show the compared values.
    ///
    /// ### Example
    /// ```no_run
    /// # #[derive(Debug, PartialEq)]
    /// # let expected_name = "name";
    /// # let actual_name = "name";
    /// # let expected_count = 1;
    /// # let actual_count = 1;
    /// assert!(actual_name == expected_name && actual_count == expected_count);
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// # #[derive(Debug, PartialEq)]
    /// # let expected_name = "name";
    /// # let actual_name = "name";
    /// # let expected_count = 1;
    /// # let actual_count = 1;
    /// assert_eq!(actual_name, expected_name);
    /// assert_eq!(actual_count, expected_count);
    /// ```
    #[clippy::version = "1.98.0"]
    pub ASSERT_MULTIPLE,
    nursery,
    "asserting multiple equality comparisons in one assertion"
}

declare_lint_pass!(AssertMultiple => [ASSERT_MULTIPLE]);

fn comparison<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<(BinOpKind, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    let ExprKind::Binary(op, lhs, rhs) = expr.kind else {
        return None;
    };

    matches!(op.node, BinOpKind::Eq | BinOpKind::Ne).then_some((op.node, lhs, rhs))
}

fn has_debug_impl(cx: &LateContext<'_>, expr: &Expr<'_>, debug_trait: rustc_hir::def_id::DefId) -> bool {
    let ty = cx.typeck_results().expr_ty(expr);
    !ty.is_raw_ptr() && implements_trait(cx, ty, debug_trait, &[])
}

fn assertion_suggestion(
    cx: &LateContext<'_>,
    assert_name: &str,
    comparison: (BinOpKind, &Expr<'_>, &Expr<'_>),
) -> String {
    let (op, lhs, rhs) = comparison;
    let suffix = match op {
        BinOpKind::Eq => "_eq",
        BinOpKind::Ne => "_ne",
        _ => unreachable!(),
    };
    format!(
        "{assert_name}{suffix}!({}, {})",
        snippet(cx, lhs.span, ".."),
        snippet(cx, rhs.span, ".."),
    )
}

impl LateLintPass<'_> for AssertMultiple {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let Some(macro_call) = root_macro_call_first_node(cx, expr) else {
            return;
        };
        let assert_name = match cx.tcx.get_diagnostic_name(macro_call.def_id) {
            Some(sym::assert_macro) => "assert",
            Some(sym::debug_assert_macro) => "debug_assert",
            _ => return,
        };
        let Some((condition, panic_call)) = find_assert_args(cx, expr, macro_call.expn) else {
            return;
        };
        if !panic_call.is_default_message() || condition.span.from_expansion() {
            return;
        }
        let ExprKind::Binary(op, lhs, rhs) = condition.kind else {
            return;
        };
        if op.node != BinOpKind::And {
            return;
        }
        let Some(lhs_comparison) = comparison(lhs) else {
            return;
        };
        let Some(rhs_comparison) = comparison(rhs) else {
            return;
        };
        let Some(debug_trait) = cx.tcx.get_diagnostic_item(sym::Debug) else {
            return;
        };
        if ![lhs_comparison.1, lhs_comparison.2, rhs_comparison.1, rhs_comparison.2]
            .into_iter()
            .all(|expr| has_debug_impl(cx, expr, debug_trait))
        {
            return;
        }

        let indent = snippet_indent(cx, macro_call.span).unwrap_or_default();
        let first = assertion_suggestion(cx, assert_name, lhs_comparison);
        let second = assertion_suggestion(cx, assert_name, rhs_comparison);
        let suggestion = format!("{first};\n{indent}{second}");
        span_lint_and_sugg(
            cx,
            ASSERT_MULTIPLE,
            macro_call.span,
            "multiple equality comparisons combined into one assertion",
            "split the comparisons into separate assertions",
            suggestion,
            Applicability::MaybeIncorrect,
        );
    }
}
