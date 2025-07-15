use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::{snippet, snippet_with_context};
use clippy_utils::{expr_use_ctxt, fn_def_id, is_trait_method, std_or_core};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Checks for `repeat().take()` that can be replaced with `repeat_n()`.
    ///
    /// ### Why is this bad?
    ///
    /// Using `repeat_n()` is more concise and clearer. Also, `repeat_n()` is sometimes faster than `repeat().take()` when the type of the element is non-trivial to clone because the original value can be reused for the last `.next()` call rather than always cloning.
    ///
    /// ### Example
    /// ```no_run
    /// let _ = std::iter::repeat(10).take(3);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let _ = std::iter::repeat_n(10, 3);
    /// ```
    #[clippy::version = "1.86.0"]
    pub MANUAL_REPEAT_N,
    style,
    "detect `repeat().take()` that can be replaced with `repeat_n()`"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    repeat_expr: &Expr<'_>,
    take_arg: &Expr<'_>,
    msrv: Msrv,
) {
    if !expr.span.from_expansion()
        && is_trait_method(cx, expr, sym::Iterator)
        && let ExprKind::Call(_, [repeat_arg]) = repeat_expr.kind
        && let Some(def_id) = fn_def_id(cx, repeat_expr)
        && cx.tcx.is_diagnostic_item(sym::iter_repeat, def_id)
        && !expr_use_ctxt(cx, expr).is_ty_unified
        && let Some(std_or_core) = std_or_core(cx)
        && msrv.meets(cx, msrvs::REPEAT_N)
    {
        let mut app = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            MANUAL_REPEAT_N,
            expr.span,
            "this `repeat().take()` can be written more concisely",
            "consider using `repeat_n()` instead",
            format!(
                "{std_or_core}::iter::repeat_n({}, {})",
                snippet_with_context(cx, repeat_arg.span, expr.span.ctxt(), "..", &mut app).0,
                snippet(cx, take_arg.span, "..")
            ),
            app,
        );
    }
}
