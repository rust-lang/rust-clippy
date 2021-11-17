use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::{find_assert_args, root_macro_call_first_node};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns about the use of inverted conditions in assert-like macros.
    ///
    /// ### Why is this bad?
    /// It is all too easy to misread the semantics of an assertion when the
    /// logic of the condition is reversed.  Explicitly comparing to a boolean
    /// value is preferable.
    ///
    /// ### Example
    /// ```rust
    /// // Bad
    /// assert!(!"a".is_empty());
    ///
    /// // Good
    /// assert_eq!("a".is_empty(), false);
    ///
    /// // Okay
    /// assert_ne!("a".is_empty(), true);
    /// ```
    #[clippy::version = "1.58.0"]
    pub BOOL_ASSERT_INVERTED,
    restriction,
    "Asserting on an inverted condition"
}

declare_lint_pass!(BoolAssertInverted => [BOOL_ASSERT_INVERTED]);

fn is_inverted(e: &Expr<'_>) -> bool {
    matches!(e.kind, ExprKind::Unary(UnOp::Not, _),) && !e.span.from_expansion()
}

impl<'tcx> LateLintPass<'tcx> for BoolAssertInverted {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let Some(macro_call) = root_macro_call_first_node(cx, expr) else { return };
        let macro_name = cx.tcx.item_name(macro_call.def_id);
        if !matches!(macro_name.as_str(), "assert" | "debug_assert") {
            return;
        }
        let Some ((a, _)) = find_assert_args(cx, expr, macro_call.expn) else { return };
        if !is_inverted(a) {
            return;
        }

        let macro_name = macro_name.as_str();
        let eq_mac = format!("{}_eq", macro_name);
        span_lint_and_sugg(
            cx,
            BOOL_ASSERT_INVERTED,
            macro_call.span,
            &format!("used `{}!` with an inverted condition", macro_name),
            "replace it with",
            format!("{}!(.., false, ..)", eq_mac),
            Applicability::MaybeIncorrect,
        );
    }
}
