use clippy_utils::{diagnostics::span_lint_and_sugg, match_def_path, paths, source::snippet};
use rustc_ast::{BorrowKind, Mutability};
use rustc_hir::ExprKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for [`core::slice::from_ref`] call with a reference.
    ///
    /// ### Why is this bad?
    /// It's more readable to just use slice literal.
    ///
    /// ### Example
    /// ```rust
    /// let x = 3;
    /// let _s = core::slice::from_ref(&x);
    /// ```
    /// Use instead:
    /// ```rust
    /// let x = 3;
    /// let _s = &[x];
    /// ```
    #[clippy::version = "1.62.0"]
    pub NEEDLESS_SLICE_FROM_REF,
    style,
    "default lint description"
}
declare_lint_pass!(NeedlessSliceFromRef => [NEEDLESS_SLICE_FROM_REF]);

impl<'tcx> LateLintPass<'tcx> for NeedlessSliceFromRef {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'_>) {
        if let ExprKind::Call(func, [arg]) = expr.kind
            // && {dbg!(func) ;true}
            && let ExprKind::Path(ref func_qpath) = func.kind
            // && {dbg!(func_qpath) ;true}
            && let Some(def_id) = cx.qpath_res(func_qpath, func.hir_id).opt_def_id()
            // && {dbg!(def_id) ;true}
            && match_def_path(cx, def_id, &paths::SLICE_FROM_REF)
            // && {dbg!("matched") ;true}
            && let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, inner) = arg.kind
        {
            span_lint_and_sugg(
                cx,
                NEEDLESS_SLICE_FROM_REF,
                expr.span,
                "needless slice::from_ref",
                "try",
                format!("&[{}]", snippet(cx, inner.span, "..")),
                rustc_errors::Applicability::MachineApplicable
            );
        }
    }
}
