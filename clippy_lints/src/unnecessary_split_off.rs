use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sym;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Suggests using `drain(..).collect()` when a `split_off(0)` is being called on a `vec`.
    /// ### Why is this bad?
    /// Because splitting implies dividing the vec into two parts, so the modified vector being emptied could be unexpected.
    /// ### Example
    /// ```no_run
    /// let mut vec = vec![1, 2, 3];
    /// let vec1 = vec.split_off(0);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let mut vec = vec![1, 2, 3];
    /// let vec1 = vec.drain(..).collect();
    /// ```
    #[clippy::version = "1.88.0"]
    pub UNNECESSARY_SPLIT_OFF,
    style,
    "unnecessary `split_off(0)`"
}
declare_lint_pass!(UnnecessarySplitOff => [UNNECESSARY_SPLIT_OFF]);

impl<'tcx> LateLintPass<'tcx> for UnnecessarySplitOff {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let ExprKind::MethodCall(path, value, args, span) = &expr.kind
        // FIXME: sym::split_off does not exist, but this still triggers the lint to use it.
            && path.ident.name.as_str() == "split_off"
        {
            let ty = cx.typeck_results().expr_ty(value);
            if clippy_utils::ty::is_type_diagnostic_item(cx, ty, sym::Vec) {
                let &[arg] = args else {
                    return;
                };
                if clippy_utils::is_integer_literal(arg, 0) || clippy_utils::is_integer_const(cx, arg, 0) {
                    span_lint_and_sugg(
                        cx,
                        UNNECESSARY_SPLIT_OFF,
                        *span,
                        "unnecessary `split_off(0)`",
                        "use",
                        "drain(..).collect()".to_string(),
                        Applicability::MachineApplicable,
                    );
                }
            }
        }
    }
}
