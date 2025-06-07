use clippy_utils::diagnostics::span_lint;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why restrict this?
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.89.0"]
    pub DIRECT_RECURSION,
    restriction,
    "default lint description"
}
declare_lint_pass!(DirectRecursion => [DIRECT_RECURSION]);

impl<'tcx> LateLintPass<'tcx> for DirectRecursion {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Call(call_expr, _) = &expr.kind
            && let body_def_id = cx.tcx.hir_enclosing_body_owner(call_expr.hir_id)
            && let ExprKind::Path(c_expr_path) = call_expr.kind
            && let QPath::Resolved(_lhs, path) = c_expr_path
            && let Res::Def(DefKind::Fn, fn_path_id) = path.res
            && fn_path_id == body_def_id.into()
        {
            span_lint(
                cx,
                DIRECT_RECURSION,
                expr.span,
                "this function contains a call to itself",
            );
        }
    }
}
