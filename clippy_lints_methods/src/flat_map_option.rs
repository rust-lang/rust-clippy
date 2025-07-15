use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_trait_method;
use clippy_utils::ty::is_type_diagnostic_item;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{Span, sym};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `Iterator::flat_map()` where `filter_map()` could be
    /// used instead.
    ///
    /// ### Why is this bad?
    /// `filter_map()` is known to always produce 0 or 1 output items per input item,
    /// rather than however many the inner iterator type produces.
    /// Therefore, it maintains the upper bound in `Iterator::size_hint()`,
    /// and communicates to the reader that the input items are not being expanded into
    /// multiple output items without their having to notice that the mapping function
    /// returns an `Option`.
    ///
    /// ### Example
    /// ```no_run
    /// let nums: Vec<i32> = ["1", "2", "whee!"].iter().flat_map(|x| x.parse().ok()).collect();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let nums: Vec<i32> = ["1", "2", "whee!"].iter().filter_map(|x| x.parse().ok()).collect();
    /// ```
    #[clippy::version = "1.53.0"]
    pub FLAT_MAP_OPTION,
    pedantic,
    "used `flat_map` where `filter_map` could be used instead"
}

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>, arg: &'tcx hir::Expr<'_>, span: Span) {
    if !is_trait_method(cx, expr, sym::Iterator) {
        return;
    }
    let arg_ty = cx.typeck_results().expr_ty_adjusted(arg);
    let sig = match arg_ty.kind() {
        ty::Closure(_, args) => args.as_closure().sig(),
        _ if arg_ty.is_fn() => arg_ty.fn_sig(cx.tcx),
        _ => return,
    };
    if !is_type_diagnostic_item(cx, sig.output().skip_binder(), sym::Option) {
        return;
    }
    span_lint_and_sugg(
        cx,
        FLAT_MAP_OPTION,
        span,
        "used `flat_map` where `filter_map` could be used instead",
        "try",
        "filter_map".into(),
        Applicability::MachineApplicable,
    );
}
