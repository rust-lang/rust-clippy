use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_lang_item;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem, is_range_literal};
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for string slices immediately followed by `as_bytes`.
    ///
    /// ### Why is this bad?
    /// It involves doing an unnecessary UTF-8 alignment check which is less efficient, and can cause a panic.
    ///
    /// ### Known problems
    /// In some cases, the UTF-8 validation and potential panic from string slicing may be required for
    /// the code's correctness. If you need to ensure the slice boundaries fall on valid UTF-8 character
    /// boundaries, the original form (`s[1..5].as_bytes()`) should be preferred.
    ///
    /// ### Example
    /// ```rust
    /// let s = "Lorem ipsum";
    /// s[1..5].as_bytes();
    /// ```
    /// Use instead:
    /// ```rust
    /// let s = "Lorem ipsum";
    /// &s.as_bytes()[1..5];
    /// ```
     #[clippy::version = "1.86.0"]
     pub SLICED_STRING_AS_BYTES,
     perf,
     "slicing a string and immediately calling as_bytes is less efficient and can lead to panics"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>) {
    if let ExprKind::Index(indexed, index, _) = recv.kind
        && is_range_literal(index)
        && let ty = cx.typeck_results().expr_ty(indexed).peel_refs()
        && (ty.is_str() || is_type_lang_item(cx, ty, LangItem::String))
    {
        let mut applicability = Applicability::MaybeIncorrect;
        let stringish = snippet_with_applicability(cx, indexed.span, "_", &mut applicability);
        let range = snippet_with_applicability(cx, index.span, "_", &mut applicability);
        span_lint_and_sugg(
            cx,
            SLICED_STRING_AS_BYTES,
            expr.span,
            "calling `as_bytes` after slicing a string",
            "try",
            format!("&{stringish}.as_bytes()[{range}]"),
            applicability,
        );
    }
}
