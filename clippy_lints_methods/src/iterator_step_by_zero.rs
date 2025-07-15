use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint;
use clippy_utils::is_trait_method;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calling `.step_by(0)` on iterators which panics.
    ///
    /// ### Why is this bad?
    /// This very much looks like an oversight. Use `panic!()` instead if you
    /// actually intend to panic.
    ///
    /// ### Example
    /// ```rust,should_panic
    /// for x in (0..100).step_by(0) {
    ///     //..
    /// }
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub ITERATOR_STEP_BY_ZERO,
    correctness,
    "using `Iterator::step_by(0)`, which will panic at runtime"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &hir::Expr<'_>, arg: &'tcx hir::Expr<'_>) {
    if is_trait_method(cx, expr, sym::Iterator)
        && let Some(Constant::Int(0)) = ConstEvalCtxt::new(cx).eval(arg)
    {
        span_lint(
            cx,
            ITERATOR_STEP_BY_ZERO,
            expr.span,
            "`Iterator::step_by(0)` will panic at runtime",
        );
    }
}
