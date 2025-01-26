use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::adjustment::Adjust;
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions that use deref coercion.
    ///
    /// ### Why is this bad?
    /// Implicit deref coercion could result in confusing behavior when writing unsafe code.
    ///
    /// ### Example
    /// ```no_run
    /// let x = &Box::new(true);
    /// let y: &bool = x;
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x = &Box::new(true);
    /// let y: &bool = x.deref();
    /// ```
    #[clippy::version = "1.86.0"]
    pub DEREF_COERCIONS,
    restriction,
    "using deref coercion"
}

declare_lint_pass!(DerefCoercions => [DEREF_COERCIONS]);

impl LateLintPass<'_> for DerefCoercions {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        let source = cx.typeck_results().expr_ty(expr).peel_refs();
        for adjustment in cx.typeck_results().expr_adjustments(expr) {
            if let Adjust::Deref(_) = adjustment.kind
                && adjustment.target.peel_refs() != source
            {
                span_lint_and_help(
                    cx,
                    DEREF_COERCIONS,
                    expr.span,
                    "implicit deref coercion",
                    None,
                    "consider using `deref() or deref_mut()`",
                );
            }
        }
    }
}
