use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::symbol::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `_.map(_).collect::<Result<(), _>()`.
    ///
    /// ### Why is this bad?
    /// Using `try_for_each` instead is more readable and idiomatic.
    ///
    /// ### Example
    /// ```no_run
    /// (0..3).map(|t| Err(t)).collect::<Result<(), _>>();
    /// ```
    /// Use instead:
    /// ```no_run
    /// (0..3).try_for_each(|t| Err(t));
    /// ```
    #[clippy::version = "1.49.0"]
    pub MAP_COLLECT_RESULT_UNIT,
    style,
    "using `.map(_).collect::<Result<(),_>()`, which can be replaced with `try_for_each`"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, iter: &hir::Expr<'_>, map_fn: &hir::Expr<'_>) {
    // return of collect `Result<(),_>`
    let collect_ret_ty = cx.typeck_results().expr_ty(expr);
    if is_type_diagnostic_item(cx, collect_ret_ty, sym::Result)
        && let ty::Adt(_, args) = collect_ret_ty.kind()
        && let Some(result_t) = args.types().next()
        && result_t.is_unit()
    // get parts for snippet
    {
        span_lint_and_sugg(
            cx,
            MAP_COLLECT_RESULT_UNIT,
            expr.span,
            "`.map().collect()` can be replaced with `.try_for_each()`",
            "try",
            format!(
                "{}.try_for_each({})",
                snippet(cx, iter.span, ".."),
                snippet(cx, map_fn.span, "..")
            ),
            Applicability::MachineApplicable,
        );
    }
}
