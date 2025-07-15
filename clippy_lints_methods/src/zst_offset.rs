use clippy_utils::diagnostics::span_lint;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `offset(_)`, `wrapping_`{`add`, `sub`}, etc. on raw pointers to
    /// zero-sized types
    ///
    /// ### Why is this bad?
    /// This is a no-op, and likely unintended
    ///
    /// ### Example
    /// ```no_run
    /// unsafe { (&() as *const ()).offset(1) };
    /// ```
    #[clippy::version = "1.41.0"]
    pub ZST_OFFSET,
    correctness,
    "Check for offset calculations on raw pointers to zero-sized types"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>) {
    if let ty::RawPtr(ty, _) = cx.typeck_results().expr_ty(recv).kind()
        && let Ok(layout) = cx.tcx.layout_of(cx.typing_env().as_query_input(*ty))
        && layout.is_zst()
    {
        span_lint(cx, ZST_OFFSET, expr.span, "offset calculation on zero-sized value");
    }
}
