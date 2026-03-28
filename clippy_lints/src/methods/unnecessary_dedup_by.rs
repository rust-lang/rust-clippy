use super::utils::{
    BindingKey, BindingSource, expr_borrows, expr_is_field_access, mapping_of_mirrored_pats, mirrored_exprs,
};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::{is_copy, peel_n_ty_refs};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Closure, Expr, ExprKind, Param, Path, PathSegment, QPath};
use rustc_lint::LateContext;
use rustc_span::{Span, sym};

use super::UNNECESSARY_DEDUP_BY;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, call_span: Span, arg: &'tcx Expr<'_>) {
    if let Some(method_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && let Some(impl_id) = cx.tcx.impl_of_assoc(method_id)
        && cx
            .tcx
            .type_of(impl_id)
            .instantiate_identity()
            .is_diag_item(cx, sym::Vec)
        && let ExprKind::Closure(&Closure { body, .. }) = arg.kind
        && let closure_body = cx.tcx.hir_body(body)
        && let &[Param { pat: l_pat, .. }, Param { pat: r_pat, .. }] = closure_body.params
        && let Some(binding_map) = mapping_of_mirrored_pats(l_pat, r_pat)
        && let ExprKind::Binary(op, left_expr, right_expr) = closure_body.value.kind
        && op.node == BinOpKind::Eq
    {
        let (key_expr, key_pat, binding_source) =
            if mirrored_exprs(left_expr, right_expr, &binding_map, BindingSource::Left) {
                (left_expr, l_pat, BindingSource::Left)
            } else if mirrored_exprs(left_expr, right_expr, &binding_map, BindingSource::Right) {
                (left_expr, r_pat, BindingSource::Right)
            } else {
                return;
            };
        let mut applicability = Applicability::MachineApplicable;
        if let ExprKind::Path(QPath::Resolved(
            _,
            Path {
                segments: [PathSegment { ident: key_name, .. }],
                ..
            },
        )) = key_expr.kind
        {
            let key_name_n_refs = binding_map
                .get(&BindingKey {
                    ident: *key_name,
                    source: binding_source,
                })
                .map_or(0, |value| value.n_refs);

            let mut key_expr_ty = cx.typeck_results().expr_ty(key_expr);
            if key_name_n_refs == 0 {
                (key_expr_ty, _) = peel_n_ty_refs(key_expr_ty, 1);
            }
            if is_copy(cx, key_expr_ty) {
                let mut key_body = snippet_with_applicability(cx, key_expr.span, "_", &mut applicability).to_string();
                if key_name_n_refs == 0 {
                    key_body = format!("*{key_body}");
                }

                let key_pat_snippet = snippet_with_applicability(cx, key_pat.span, "_", &mut applicability);

                span_lint_and_then(
                    cx,
                    UNNECESSARY_DEDUP_BY,
                    expr.span,
                    "consider using `dedup_by_key`",
                    |diag| {
                        diag.span_suggestion_verbose(
                            call_span,
                            "try",
                            format!("dedup_by_key(|{key_pat_snippet}| {key_body})"),
                            applicability,
                        );
                    },
                );
            }
        } else {
            let key_expr_ty = cx.typeck_results().expr_ty(key_expr);
            if !expr_borrows(key_expr_ty) && (!expr_is_field_access(key_expr) || is_copy(cx, key_expr_ty)) {
                let key_body = snippet_with_applicability(cx, key_expr.span, "_", &mut applicability).to_string();
                let key_pat_snippet = snippet_with_applicability(cx, key_pat.span, "_", &mut applicability);

                span_lint_and_then(
                    cx,
                    UNNECESSARY_DEDUP_BY,
                    expr.span,
                    "consider using `dedup_by_key`",
                    |diag| {
                        diag.span_suggestion_verbose(
                            call_span,
                            "try",
                            format!("dedup_by_key(|{key_pat_snippet}| {key_body})"),
                            applicability,
                        );
                    },
                );
            }
        }
    }
}
