use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::implements_trait;
use clippy_utils::{get_parent_expr, sym};
use rustc_errors::Applicability;
use rustc_hir::{ExprKind, Pat, PatKind, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::MATCHES_INSTEAD_OF_EQ;

fn snippet_and_ty_of_arm<'tcx>(cx: &LateContext<'tcx>, arm: &rustc_hir::Arm<'_>) -> Option<(String, Ty<'tcx>)> {
    if let PatKind::Expr(expr) = arm.pat.kind
        && let Some(snippet) = snippet_opt(cx, expr.span)
        && let Some(ty) = cx.typeck_results().node_type_opt(expr.hir_id)
    {
        Some((snippet, ty))
    } else {
        None
    }
}

fn get_correct_eq_span(cx: &LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> (Span, &'static str) {
    if let Some(parent) = get_parent_expr(cx, expr)
        && let ExprKind::Unary(UnOp::Not, _) = parent.kind
    {
        (parent.span.source_callsite(), "!=")
    } else {
        (expr.span.source_callsite(), "==")
    }
}

// Returns `true` if the pattern can be used in a `==` comparison.
fn is_valid_pattern_for_eq(pat: &Pat<'_>) -> bool {
    match pat.kind {
        PatKind::Or(..)
        | PatKind::Struct(..)
        | PatKind::TupleStruct(..)
        | PatKind::Never
        | PatKind::Binding(..)
        | PatKind::Wild
        | PatKind::Missing
        | PatKind::Guard(..)
        | PatKind::Err(_)
        | PatKind::Slice(_, Some(_), _) => false,
        PatKind::Slice(part1, None, part2) => {
            part1.iter().all(|p| is_valid_pattern_for_eq(p)) && part2.iter().all(|p| is_valid_pattern_for_eq(p))
        },
        PatKind::Ref(pat, _, _) | PatKind::Deref(pat) | PatKind::Box(pat) => is_valid_pattern_for_eq(pat),
        PatKind::Tuple(pats, dotdot_pos) => {
            // If `dotdot_pos <= pat.len`, then it means there is a `..`.
            dotdot_pos.as_opt_usize().unwrap_or(0) > pats.len() && pats.iter().all(|p| is_valid_pattern_for_eq(p))
        },
        PatKind::Expr(_) | PatKind::Range(..) => true,
    }
}

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &rustc_hir::Expr<'_>,
    ex: &rustc_hir::Expr<'_>,
    arms: &[rustc_hir::Arm<'_>],
) {
    let Some(partial_eq_def_id) = cx.tcx.get_diagnostic_item(sym::PartialEq) else {
        return;
    };
    let mut expr_ty = cx.typeck_results().expr_ty(ex);
    if !implements_trait(cx, expr_ty, partial_eq_def_id, &[expr_ty.into()]) {
        return;
    }
    if arms
        .iter()
        .any(|arm| is_valid_pattern_for_eq(arm.pat) && arm.guard.is_none())
        && let Some((snippet2, mut ty2)) = arms.iter().find_map(|arm| snippet_and_ty_of_arm(cx, arm))
        && let ExprKind::Match(m, ..) = expr.kind
        && let Some(snippet1) = snippet_opt(cx, m.span)
    {
        let (span, comparison) = get_correct_eq_span(cx, expr);
        let mut derefs1 = String::new();
        let mut derefs2 = String::new();
        while !implements_trait(cx, expr_ty, partial_eq_def_id, &[ty2.into()]) {
            if let ty::Ref(_, inner_ty, _) = expr_ty.kind() {
                expr_ty = *inner_ty;
                derefs1.push('*');
            } else if let ty::Ref(_, inner_ty, _) = ty2.kind() {
                ty2 = *inner_ty;
                derefs2.push('*');
            } else {
                // Hum... okay?
                return;
            }
        }

        cx.tcx.for_each_relevant_impl(partial_eq_def_id, expr_ty, |impl_id| {
            if !cx.tcx.is_automatically_derived(impl_id) {
                return;
            }

            let trait_ref = cx.tcx.impl_trait_ref(impl_id);
            if trait_ref.instantiate_identity().skip_norm_wip().args.type_at(1) != ty2 {
                return;
            }
            span_lint_and_sugg(
                cx,
                MATCHES_INSTEAD_OF_EQ,
                span,
                format!("this expression can be replaced with a `{comparison}` comparison"),
                "replace with",
                format!("{derefs1}{snippet1} {comparison} {derefs2}{snippet2}"),
                Applicability::MaybeIncorrect,
            );
        });
    }
}
