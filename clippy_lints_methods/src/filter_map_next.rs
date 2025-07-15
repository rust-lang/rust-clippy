use clippy_utils::diagnostics::{span_lint, span_lint_and_sugg};
use clippy_utils::is_trait_method;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `_.filter_map(_).next()`.
    ///
    /// ### Why is this bad?
    /// Readability, this can be written more concisely as
    /// `_.find_map(_)`.
    ///
    /// ### Example
    /// ```no_run
    ///  (0..3).filter_map(|x| if x == 2 { Some(x) } else { None }).next();
    /// ```
    /// Can be written as
    ///
    /// ```no_run
    ///  (0..3).find_map(|x| if x == 2 { Some(x) } else { None });
    /// ```
    #[clippy::version = "1.36.0"]
    pub FILTER_MAP_NEXT,
    pedantic,
    "using combination of `filter_map` and `next` which can usually be written as a single method call"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'_>,
    recv: &'tcx hir::Expr<'_>,
    arg: &'tcx hir::Expr<'_>,
    msrv: Msrv,
) {
    if is_trait_method(cx, expr, sym::Iterator) {
        if !msrv.meets(cx, msrvs::ITERATOR_FIND_MAP) {
            return;
        }

        let msg = "called `filter_map(..).next()` on an `Iterator`. This is more succinctly expressed by calling \
                   `.find_map(..)` instead";
        let filter_snippet = snippet(cx, arg.span, "..");
        if filter_snippet.lines().count() <= 1 {
            let iter_snippet = snippet(cx, recv.span, "..");
            span_lint_and_sugg(
                cx,
                FILTER_MAP_NEXT,
                expr.span,
                msg,
                "try",
                format!("{iter_snippet}.find_map({filter_snippet})"),
                Applicability::MachineApplicable,
            );
        } else {
            span_lint(cx, FILTER_MAP_NEXT, expr.span, msg);
        }
    }
}
