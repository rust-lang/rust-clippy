use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::ty::implements_trait;
use rustc_errors::Applicability;
use rustc_middle::ty;
use rustc_span::sym;
use clippy_utils::ty::is_type_diagnostic_item;

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
    #[clippy::version = "1.91.0"]
    pub DISCARDED_FUTURE,
    restriction,
    "default lint description"
}
declare_lint_pass!(DiscardedFuture => [DISCARDED_FUTURE]);

impl<'tcx> LateLintPass<'tcx> for DiscardedFuture {
    fn check_stmt(&mut self, cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) {
        if let StmtKind::Let(let_stmt) = stmt.kind
            && let PatKind::Wild = let_stmt.pat.kind
            && let Some(expr) = let_stmt.init
            && let ty = cx.typeck_results().expr_ty(expr)
            && is_type_diagnostic_item(cx, ty, sym::Result)
            && let ty::Adt(_, substs) = ty.kind()
            && let Some(inner_ty) = substs[0].as_type()
            && let Some(future_trait_def_id) = cx.tcx.lang_items().future_trait()
            && implements_trait(cx, inner_ty, future_trait_def_id, &[])
        {
            span_lint_and_sugg(
                cx,
                DISCARDED_FUTURE,
                expr.span,
                format!("Discarding a Result<Future>: did you mean to call .await on this first?"),
                "consider `.await` on it",
                format!("{}.await", snippet(cx, expr.span, "..")),
                Applicability::MaybeIncorrect,
            );
        }
    }
}