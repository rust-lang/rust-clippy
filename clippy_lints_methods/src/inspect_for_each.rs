use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_trait_method;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `inspect().for_each()`.
    ///
    /// ### Why is this bad?
    /// It is the same as performing the computation
    /// inside `inspect` at the beginning of the closure in `for_each`.
    ///
    /// ### Example
    /// ```no_run
    /// [1,2,3,4,5].iter()
    /// .inspect(|&x| println!("inspect the number: {}", x))
    /// .for_each(|&x| {
    ///     assert!(x >= 0);
    /// });
    /// ```
    /// Can be written as
    /// ```no_run
    /// [1,2,3,4,5].iter()
    /// .for_each(|&x| {
    ///     println!("inspect the number: {}", x);
    ///     assert!(x >= 0);
    /// });
    /// ```
    #[clippy::version = "1.51.0"]
    pub INSPECT_FOR_EACH,
    complexity,
    "using `.inspect().for_each()`, which can be replaced with `.for_each()`"
}

/// lint use of `inspect().for_each()` for `Iterators`
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>, inspect_span: Span) {
    if is_trait_method(cx, expr, sym::Iterator) {
        let msg = "called `inspect(..).for_each(..)` on an `Iterator`";
        let hint = "move the code from `inspect(..)` to `for_each(..)` and remove the `inspect(..)`";
        span_lint_and_help(
            cx,
            INSPECT_FOR_EACH,
            inspect_span.with_hi(expr.span.hi()),
            msg,
            None,
            hint,
        );
    }
}
