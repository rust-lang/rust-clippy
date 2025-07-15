use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sym;
use clippy_utils::ty::{is_type_diagnostic_item, is_type_lang_item};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::LateContext;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for occurrences where one vector gets extended instead of append
    ///
    /// ### Why is this bad?
    /// Using `append` instead of `extend` is more concise and faster
    ///
    /// ### Example
    /// ```no_run
    /// let mut a = vec![1, 2, 3];
    /// let mut b = vec![4, 5, 6];
    ///
    /// a.extend(b.drain(..));
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// let mut a = vec![1, 2, 3];
    /// let mut b = vec![4, 5, 6];
    ///
    /// a.append(&mut b);
    /// ```
    #[clippy::version = "1.55.0"]
    pub EXTEND_WITH_DRAIN,
    perf,
    "using vec.append(&mut vec) to move the full range of a vector to another"
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, arg: &Expr<'_>) {
    let ty = cx.typeck_results().expr_ty(recv).peel_refs();
    if is_type_diagnostic_item(cx, ty, sym::Vec)
        //check source object
        && let ExprKind::MethodCall(src_method, drain_vec, [drain_arg], _) = &arg.kind
        && src_method.ident.name == sym::drain
        && let src_ty = cx.typeck_results().expr_ty(drain_vec)
        //check if actual src type is mutable for code suggestion
        && let immutable = src_ty.is_mutable_ptr()
        && let src_ty = src_ty.peel_refs()
        && is_type_diagnostic_item(cx, src_ty, sym::Vec)
        //check drain range
        && let src_ty_range = cx.typeck_results().expr_ty(drain_arg).peel_refs()
        && is_type_lang_item(cx, src_ty_range, LangItem::RangeFull)
    {
        let mut applicability = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            EXTEND_WITH_DRAIN,
            expr.span,
            "use of `extend` instead of `append` for adding the full range of a second vector",
            "try",
            format!(
                "{}.append({}{})",
                snippet_with_applicability(cx, recv.span, "..", &mut applicability),
                if immutable { "" } else { "&mut " },
                snippet_with_applicability(cx, drain_vec.span, "..", &mut applicability)
            ),
            applicability,
        );
    }
}
