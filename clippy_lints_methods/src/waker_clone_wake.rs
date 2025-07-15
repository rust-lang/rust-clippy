use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_trait_method;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `waker.clone().wake()`
    ///
    /// ### Why is this bad?
    /// Cloning the waker is not necessary, `wake_by_ref()` enables the same operation
    /// without extra cloning/dropping.
    ///
    /// ### Example
    /// ```rust,ignore
    /// waker.clone().wake();
    /// ```
    /// Should be written
    /// ```rust,ignore
    /// waker.wake_by_ref();
    /// ```
    #[clippy::version = "1.75.0"]
    pub WAKER_CLONE_WAKE,
    perf,
    "cloning a `Waker` only to wake it"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, recv: &'tcx Expr<'_>) {
    let ty = cx.typeck_results().expr_ty(recv);

    if let Some(did) = ty.ty_adt_def()
        && cx.tcx.is_diagnostic_item(sym::Waker, did.did())
        && let ExprKind::MethodCall(_, waker_ref, &[], _) = recv.kind
        && is_trait_method(cx, recv, sym::Clone)
    {
        let mut applicability = Applicability::MachineApplicable;
        let snippet = snippet_with_applicability(cx, waker_ref.span.source_callsite(), "..", &mut applicability);

        span_lint_and_sugg(
            cx,
            WAKER_CLONE_WAKE,
            expr.span,
            "cloning a `Waker` only to wake it",
            "replace with",
            format!("{snippet}.wake_by_ref()"),
            applicability,
        );
    }
}
