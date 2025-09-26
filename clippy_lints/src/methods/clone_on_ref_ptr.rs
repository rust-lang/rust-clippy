use clippy_utils::diagnostics::{applicability_for_ctxt, span_lint_and_then};
use clippy_utils::source::SpanExt;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::symbol::{Symbol, sym};

use super::CLONE_ON_REF_PTR;

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &hir::Expr<'_>,
    method_name: Symbol,
    receiver: &hir::Expr<'_>,
    args: &[hir::Expr<'_>],
) {
    if !(args.is_empty() && method_name == sym::clone) {
        return;
    }
    let obj_ty = cx.typeck_results().expr_ty(receiver).peel_refs();

    if let ty::Adt(adt, subst) = obj_ty.kind()
        && let Some(name) = cx.tcx.get_diagnostic_name(adt.did())
        && let caller_type = match name {
            sym::Rc => "std::rc::Rc",
            sym::Arc => "std::sync::Arc",
            sym::RcWeak => "std::rc::Weak",
            sym::ArcWeak => "std::sync::Weak",
            _ => return,
        }
        && let ctxt = expr.span.ctxt()
        && let Some(snippet) = receiver.span.get_source_text_at_ctxt(cx, ctxt)
    {
        span_lint_and_then(
            cx,
            CLONE_ON_REF_PTR,
            expr.span,
            "using `.clone()` on a ref-counted pointer",
            |diag| {
                // Sometimes unnecessary ::<_> after Rc/Arc/Weak
                diag.span_suggestion(
                    expr.span,
                    "try",
                    format!("{caller_type}::<{}>::clone(&{snippet})", subst.type_at(0)),
                    applicability_for_ctxt(ctxt),
                );
            },
        );
    }
}
