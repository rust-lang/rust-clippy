use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_lang_item;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// It checks for `str::bytes().count()` and suggests replacing it with
    /// `str::len()`.
    ///
    /// ### Why is this bad?
    /// `str::bytes().count()` is longer and may not be as performant as using
    /// `str::len()`.
    ///
    /// ### Example
    /// ```no_run
    /// "hello".bytes().count();
    /// String::from("hello").bytes().count();
    /// ```
    /// Use instead:
    /// ```no_run
    /// "hello".len();
    /// String::from("hello").len();
    /// ```
    #[clippy::version = "1.62.0"]
    pub BYTES_COUNT_TO_LEN,
    complexity,
    "Using `bytes().count()` when `len()` performs the same functionality"
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'_>,
    count_recv: &'tcx hir::Expr<'_>,
    bytes_recv: &'tcx hir::Expr<'_>,
) {
    if let Some(bytes_id) = cx.typeck_results().type_dependent_def_id(count_recv.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_method(bytes_id)
        && cx.tcx.type_of(impl_id).instantiate_identity().is_str()
        && let ty = cx.typeck_results().expr_ty(bytes_recv).peel_refs()
        && (ty.is_str() || is_type_lang_item(cx, ty, hir::LangItem::String))
    {
        let mut applicability = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            BYTES_COUNT_TO_LEN,
            expr.span,
            "using long and hard to read `.bytes().count()`",
            "consider calling `.len()` instead",
            format!(
                "{}.len()",
                snippet_with_applicability(cx, bytes_recv.span, "..", &mut applicability)
            ),
            applicability,
        );
    }
}
