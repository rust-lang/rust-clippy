use clippy_utils::{diagnostics::span_lint_and_sugg, source::snippet_with_applicability, ty::is_type_lang_item};
use rustc_errors::Applicability;
use rustc_hir::{
    Expr, ExprKind,
    LangItem::{self, Range, RangeFrom, RangeFull, RangeTo, RangeToInclusive},
    QPath,
};
use rustc_lint::LateContext;

use super::SLICE_AS_BYTES;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>) {
    if let ExprKind::Index(indexed, index) = recv.kind {
        if let ExprKind::Struct(QPath::LangItem(Range | RangeFrom | RangeTo | RangeToInclusive | RangeFull, ..), ..) =
            index.kind
        {
            let ty = cx.typeck_results().expr_ty(indexed).peel_refs();
            let is_str = ty.is_str();
            let is_string = is_type_lang_item(cx, ty, LangItem::String);
            if is_str || is_string {
                let mut applicability = Applicability::MachineApplicable;
                let stringish = snippet_with_applicability(cx, indexed.span, "..", &mut applicability);
                let range = snippet_with_applicability(cx, index.span, "..", &mut applicability);
                let type_name = if is_str { "str" } else { "String" };
                span_lint_and_sugg(
                    cx,
                    SLICE_AS_BYTES,
                    expr.span,
                    &(format!(
                        "slicing a {type_name} before calling `as_bytes` results in needless UTF-8 alignment checks, and has the possiblity of panicking"
                    )),
                    "try",
                    format!("&{stringish}.as_bytes()[{range}]"),
                    applicability,
                );
            }
        }
    }
}
