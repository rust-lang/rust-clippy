use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::{is_from_proc_macro, is_trait_method};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.skip(0)` on iterators.
    ///
    /// ### Why is this bad?
    /// This was likely intended to be `.skip(1)` to skip the first element, as `.skip(0)` does
    /// nothing. If not, the call should be removed.
    ///
    /// ### Example
    /// ```no_run
    /// let v = vec![1, 2, 3];
    /// let x = v.iter().skip(0).collect::<Vec<_>>();
    /// let y = v.iter().collect::<Vec<_>>();
    /// assert_eq!(x, y);
    /// ```
    #[clippy::version = "1.73.0"]
    pub ITER_SKIP_ZERO,
    correctness,
    "disallows `.skip(0)`"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, arg_expr: &Expr<'_>) {
    if !expr.span.from_expansion()
        && is_trait_method(cx, expr, sym::Iterator)
        && let Some(arg) = ConstEvalCtxt::new(cx).eval(arg_expr).and_then(|constant| {
            if let Constant::Int(arg) = constant {
                Some(arg)
            } else {
                None
            }
        })
        && arg == 0
        && !is_from_proc_macro(cx, expr)
    {
        span_lint_and_then(cx, ITER_SKIP_ZERO, arg_expr.span, "usage of `.skip(0)`", |diag| {
            diag.span_suggestion(
                arg_expr.span,
                "if you meant to skip the first element, use",
                "1",
                Applicability::MaybeIncorrect,
            )
            .note("this call to `skip` does nothing and is useless; remove it");
        });
    }
}
