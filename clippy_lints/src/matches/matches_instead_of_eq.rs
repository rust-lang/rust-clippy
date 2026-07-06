use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_opt;
use clippy_utils::ty::implements_trait;
use clippy_utils::{get_parent_expr, sym};
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::{ExprKind, Pat, PatKind, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::MATCHES_INSTEAD_OF_EQ;

fn snippet_and_ty_of_arm<'tcx>(cx: &LateContext<'tcx>, arm: &rustc_hir::Arm<'_>) -> Option<(String, Ty<'tcx>)> {
    let (span, hir_id) = match arm.pat.kind {
        PatKind::Expr(expr) => (expr.span, expr.hir_id),
        PatKind::TupleStruct(..) => (arm.pat.span, arm.pat.hir_id),
        _ => return None,
    };
    if let Some(snippet) = snippet_opt(cx, span)
        && let Some(ty) = cx.typeck_results().node_type_opt(hir_id)
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
        PatKind::Tuple(pats, dotdot_pos) | PatKind::TupleStruct(_, pats, dotdot_pos) => {
            dotdot_pos.as_opt_usize().is_none() && pats.iter().all(|p| is_valid_pattern_for_eq(p))
        },
        PatKind::Expr(_) | PatKind::Range(..) => true,
    }
}

fn is_deriving_partial_eq<'tcx>(tcx: TyCtxt<'tcx>, partial_eq_def_id: DefId, ty: Ty<'tcx>) -> bool {
    let mut is_automatically_derived = false;
    tcx.for_each_relevant_impl(partial_eq_def_id, ty, |impl_id| {
        is_automatically_derived = is_automatically_derived || tcx.is_automatically_derived(impl_id);
    });
    is_automatically_derived
}

fn is_fully_automatically_derived<'tcx>(
    cx: &LateContext<'tcx>,
    ty: Ty<'tcx>,
    automatically_derived_partial_eq_map: &mut FxHashMap<Ty<'tcx>, bool>,
    partial_eq_def_id: DefId,
) -> bool {
    if let Some(is_automatically_derived) = automatically_derived_partial_eq_map.get(&ty) {
        return *is_automatically_derived;
    }
    if !is_deriving_partial_eq(cx.tcx, partial_eq_def_id, ty) {
        automatically_derived_partial_eq_map.insert(ty, false);
        return false;
    }
    let (adt, generics) = match ty.peel_refs().kind() {
        ty::Adt(adt_def, generics) => (adt_def, generics),
        ty::Bool
        | ty::Char
        | ty::Int(_)
        | ty::Uint(_)
        | ty::Float(_)
        | ty::Str
        | ty::RawPtr(..)
        | ty::FnDef(..)
        | ty::Never
        | ty::FnPtr(..) => return true,
        ty::Array(ty, _) | ty::Ref(_, ty, _) | ty::Slice(ty) => {
            return is_fully_automatically_derived(cx, *ty, automatically_derived_partial_eq_map, partial_eq_def_id);
        },
        ty::Tuple(tys) => {
            return tys.iter().all(|ty| {
                is_fully_automatically_derived(cx, ty, automatically_derived_partial_eq_map, partial_eq_def_id)
            });
        },
        ty::Pat(..)
        | ty::Alias(..) // FIXME: Should be handled correctly
        | ty::Foreign(..) // FIXME: Should be handled correctly
        | ty::Dynamic(..)
        | ty::Param(..)
        | ty::Bound(..)
        | ty::Placeholder(..)
        | ty::Infer(..)
        | ty::Error(..)
        | ty::Closure(..)
        | ty::CoroutineClosure(..)
        | ty::UnsafeBinder(..)
        | ty::CoroutineWitness(..)
        | ty::Coroutine(..) => return false,
    };
    for variant in adt.variants() {
        for field in &variant.fields {
            let field_ty = field.ty(cx.tcx, generics).skip_norm_wip();
            if let Some(is_automatically_derived) = automatically_derived_partial_eq_map.get(&field_ty) {
                // We stop at the first occurrence of non-automatically derived `ty`.
                if !*is_automatically_derived {
                    // We add a new entry into our map for this type.
                    automatically_derived_partial_eq_map.insert(ty, false);
                    return false;
                }
                continue;
            }
            if !is_deriving_partial_eq(cx.tcx, partial_eq_def_id, field_ty) {
                // We add a new entry into our map for both this type and the field's type.
                automatically_derived_partial_eq_map.insert(ty, false);
                automatically_derived_partial_eq_map.insert(field_ty, false);
                return false;
            }
            automatically_derived_partial_eq_map.insert(field_ty, true);
        }
    }
    automatically_derived_partial_eq_map.insert(ty, true);
    true
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &rustc_hir::Expr<'_>,
    ex: &rustc_hir::Expr<'_>,
    arms: &[rustc_hir::Arm<'_>],
    automatically_derived_partial_eq_map: &mut FxHashMap<Ty<'tcx>, bool>,
) {
    let Some(partial_eq_def_id) = cx.tcx.get_diagnostic_item(sym::PartialEq) else {
        return;
    };
    let mut expr_ty = cx.typeck_results().expr_ty(ex);
    if !implements_trait(cx, expr_ty, partial_eq_def_id, &[expr_ty.into()]) {
        return;
    }
    if let ExprKind::Match(m, ..) = expr.kind
        && arms
            .iter()
            .any(|arm| arm.guard.is_none() && is_valid_pattern_for_eq(arm.pat))
        && let Some((snippet2, mut ty2)) = arms.iter().find_map(|arm| snippet_and_ty_of_arm(cx, arm))
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

        if !is_fully_automatically_derived(cx, expr_ty, automatically_derived_partial_eq_map, partial_eq_def_id) {
            return;
        }

        cx.tcx.for_each_relevant_impl(partial_eq_def_id, expr_ty, |impl_id| {
            if !cx.tcx.is_automatically_derived(impl_id) {
                return;
            }

            let trait_ref = cx.tcx.impl_trait_ref(impl_id);
            if trait_ref.instantiate_identity().skip_norm_wip().args.type_at(1) != ty2 {
                return;
            }
            if !is_fully_automatically_derived(cx, ty2, automatically_derived_partial_eq_map, partial_eq_def_id) {
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
