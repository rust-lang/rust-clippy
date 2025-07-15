use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::usage::mutated_variables;
use clippy_utils::{expr_or_init, is_trait_method};
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for calls to `map` followed by a `count`.
    ///
    /// ### Why is this bad?
    /// It looks suspicious. Maybe `map` was confused with `filter`.
    /// If the `map` call is intentional, this should be rewritten
    /// using `inspect`. Or, if you intend to drive the iterator to
    /// completion, you can just use `for_each` instead.
    ///
    /// ### Example
    /// ```no_run
    /// let _ = (0..3).map(|x| x + 2).count();
    /// ```
    #[clippy::version = "1.39.0"]
    pub SUSPICIOUS_MAP,
    suspicious,
    "suspicious usage of map"
}

pub fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, count_recv: &hir::Expr<'_>, map_arg: &hir::Expr<'_>) {
    if is_trait_method(cx, count_recv, sym::Iterator)
        && let hir::ExprKind::Closure(closure) = expr_or_init(cx, map_arg).kind
        && let closure_body = cx.tcx.hir_body(closure.body)
        && !cx.typeck_results().expr_ty(closure_body.value).is_unit()
    {
        if let Some(map_mutated_vars) = mutated_variables(closure_body.value, cx)
            // A variable is used mutably inside of the closure. Suppress the lint.
            && !map_mutated_vars.is_empty()
        {
            return;
        }
        span_lint_and_help(
            cx,
            SUSPICIOUS_MAP,
            expr.span,
            "this call to `map()` won't have an effect on the call to `count()`",
            None,
            "make sure you did not confuse `map` with `filter`, `for_each` or `inspect`",
        );
    }
}
