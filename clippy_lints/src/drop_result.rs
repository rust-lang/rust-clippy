use clippy_utils::diagnostics::span_lint;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `std::mem::drop(t)` where `t` is
    /// a `Result`.
    ///
    /// ### Why is this bad?
    /// Dropping a `Result` means that its value wasn't checked.
    ///
    /// ### Example
    /// ```no_run
    /// # use std::mem;
    /// mem::drop(Err::<(), &str>("something went wrong"));
    /// ```
    #[clippy::version = "1.76.0"]
    pub DROP_RESULT,
    pedantic,
    "`mem::drop` usage on `Result` types"
}

declare_lint_pass!(DropResult => [
    DROP_RESULT,
]);

impl<'tcx> LateLintPass<'tcx> for DropResult {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Call(path, [arg]) = expr.kind
            && let ExprKind::Path(ref qpath) = path.kind
            && let Some(def_id) = cx.qpath_res(qpath, path.hir_id).opt_def_id()
            && let Some(fn_name) = cx.tcx.get_diagnostic_name(def_id)
            && fn_name == sym::mem_drop
            && let arg_ty = cx.typeck_results().expr_ty(arg)
            && is_type_diagnostic_item(cx, arg_ty, sym::Result)
        {
            span_lint(cx, DROP_RESULT, expr.span, "using `drop()` on a `Result` type");
        }
    }
}
