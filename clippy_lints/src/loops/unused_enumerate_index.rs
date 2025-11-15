use super::UNUSED_ENUMERATE_INDEX;
use clippy_utils::diagnostics::span_lint_hir_and_then;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::{FileRangeExt, SpanExt};
use clippy_utils::{expr_or_init, pat_is_wild};
use rustc_errors::Applicability;
use rustc_hir::{Closure, Expr, ExprKind, Pat, PatKind, TyKind};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    iter_expr: &'tcx Expr<'tcx>,
    pat: &Pat<'tcx>,
    ty_spans: Option<(Span, Span)>,
    body: &'tcx Expr<'tcx>,
) {
    if let PatKind::Tuple([idx_pat, inner_pat], _) = pat.kind
        && cx.typeck_results().expr_ty(iter_expr).is_diag_item(cx, sym::Enumerate)
        && pat_is_wild(cx, &idx_pat.kind, body)
        && let enumerate_call = expr_or_init(cx, iter_expr)
        && let ExprKind::MethodCall(_, _, [], enumerate_span) = enumerate_call.kind
        && let Some(enumerate_id) = cx.typeck_results().type_dependent_def_id(enumerate_call.hir_id)
        && cx.tcx.is_diagnostic_item(sym::enumerate_method, enumerate_id)
        && !enumerate_call.span.from_expansion()
        && !pat.span.from_expansion()
        && !idx_pat.span.from_expansion()
        && !inner_pat.span.from_expansion()
        && let Some(enumerate_span) = enumerate_span.map_range(cx, |scx, range| range.with_leading_match(scx, '.'))
    {
        span_lint_hir_and_then(
            cx,
            UNUSED_ENUMERATE_INDEX,
            enumerate_call.hir_id,
            enumerate_span,
            "you seem to use `.enumerate()` and immediately discard the index",
            |diag| {
                let mut spans = Vec::with_capacity(5);
                spans.push((enumerate_span, String::new()));
                spans.push((pat.span.with_hi(inner_pat.span.lo()), String::new()));
                spans.push((pat.span.with_lo(inner_pat.span.hi()), String::new()));
                if let Some((outer, inner)) = ty_spans {
                    spans.push((outer.with_hi(inner.lo()), String::new()));
                    spans.push((outer.with_lo(inner.hi()), String::new()));
                }
                diag.multipart_suggestion(
                    "remove the `.enumerate()` call",
                    spans,
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

pub(super) fn check_method<'tcx>(
    cx: &LateContext<'tcx>,
    recv: &'tcx Expr<'tcx>,
    arg: &'tcx Expr<'tcx>,
    closure: &'tcx Closure<'tcx>,
) {
    let body = cx.tcx.hir_body(closure.body);
    if let [param] = body.params
        && let [input] = closure.fn_decl.inputs
        && !arg.span.from_expansion()
        && !input.span.from_expansion()
        && !recv.span.from_expansion()
        && !param.span.from_expansion()
    {
        let ty_spans = if let TyKind::Tup([_, inner]) = input.kind {
            Some((input.span, inner.span.walk_to_root()))
        } else {
            None
        };
        check(cx, recv, param.pat, ty_spans, body.value);
    }
}
