use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `as_str()` on a `String` chained with a method available on the `String` itself.
    ///
    /// ### Why is this bad?
    /// The `as_str()` conversion is pointless and can be removed for simplicity and cleanliness.
    ///
    /// ### Example
    /// ```no_run
    /// let owned_string = "This is a string".to_owned();
    /// owned_string.as_str().as_bytes()
    /// # ;
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let owned_string = "This is a string".to_owned();
    /// owned_string.as_bytes()
    /// # ;
    /// ```
    #[clippy::version = "1.74.0"]
    pub REDUNDANT_AS_STR,
    complexity,
    "`as_str` used to call a method on `str` that is also available on `String`"
}

pub(super) fn check(
    cx: &LateContext<'_>,
    _expr: &Expr<'_>,
    recv: &Expr<'_>,
    as_str_span: Span,
    other_method_span: Span,
) {
    if cx
        .typeck_results()
        .expr_ty(recv)
        .ty_adt_def()
        .is_some_and(|adt| Some(adt.did()) == cx.tcx.lang_items().string())
    {
        let mut applicability = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            REDUNDANT_AS_STR,
            as_str_span.to(other_method_span),
            "this `as_str` is redundant and can be removed as the method immediately following exists on `String` too",
            "try",
            snippet_with_applicability(cx, other_method_span, "..", &mut applicability).into_owned(),
            applicability,
        );
    }
}
