use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::sym;
use rustc_attr_ir::lang_items::LangItem;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;

use super::EXTEND_WITH_DRAIN;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, extend_receiver: &Expr<'_>, arg: &Expr<'_>) {
    // Check whether the argument of the `extend()` is a `drain()` first, because it’s cheaper than checking types.
    if let ExprKind::MethodCall(src_method, drain_vec, [drain_arg], _) = &arg.kind
        && src_method.ident.name == sym::drain
    {
        let ty = cx.typeck_results().expr_ty(extend_receiver).peel_refs();
        if ty.is_diag_item(cx, sym::Vec)
            && let src_ty = cx.typeck_results().expr_ty(drain_vec)
            && src_ty.peel_refs().is_diag_item(cx, sym::Vec)
            //check drain range
            && let src_ty_range = cx.typeck_results().expr_ty(drain_arg).peel_refs()
            && src_ty_range.is_lang_item(cx, LangItem::RangeFull)
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
                    snippet_with_applicability(cx, extend_receiver.span, "..", &mut applicability),
                    if src_ty.is_mutable_ptr() { "" } else { "&mut " },
                    snippet_with_applicability(cx, drain_vec.span, "..", &mut applicability)
                ),
                applicability,
            );
        }
    }
}
