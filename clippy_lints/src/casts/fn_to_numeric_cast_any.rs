use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_in_const_context;
use clippy_utils::source::snippet_with_context;
use clippy_utils::sugg::Sugg;
use rustc_errors::Applicability;
use rustc_hir::def::{CtorKind, DefKind};
use rustc_hir::{Expr, ExprKind, MatchSource};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty, Unnormalized};

use super::FN_TO_NUMERIC_CAST_ANY;

fn can_cast_to_int<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    match *ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => true,
        ty::RawPtr(pointee, _) => pointee.is_sized(cx.tcx, cx.typing_env()),
        ty::Adt(adt, _) => {
            adt.is_enum()
                && !adt.variants().is_empty()
                && !adt.has_dtor(cx.tcx)
                && adt
                    .variants()
                    .iter()
                    .all(|v| matches!(v.ctor_kind(), Some(CtorKind::Const)))
        },
        _ => false,
    }
}

fn can_suggest_invoke<'tcx>(cx: &LateContext<'tcx>, cast_from: Ty<'tcx>, cast_to: Ty<'tcx>) -> bool {
    if !cast_to.is_integral() || is_in_const_context(cx) {
        return false;
    }

    let fn_sig = cx.tcx.instantiate_bound_regions_with_erased(cast_from.fn_sig(cx.tcx));
    if !fn_sig.inputs().is_empty() || fn_sig.c_variadic() || fn_sig.safety().is_unsafe() {
        return false;
    }

    if let ty::FnDef(def_id, _) = *cast_from.kind()
        && (cx.tcx.def_kind(def_id) == DefKind::Fn || cx.tcx.impl_of_assoc(def_id).is_some())
        && !cx.tcx.codegen_fn_attrs(def_id).target_features.is_empty()
    {
        return false;
    }

    let output = cx
        .tcx
        .try_normalize_erasing_regions(cx.typing_env(), Unnormalized::new_wip(fn_sig.output()))
        .unwrap_or_else(|_| fn_sig.output());

    can_cast_to_int(cx, output)
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'_>,
    cast_expr: &Expr<'_>,
    cast_from: Ty<'tcx>,
    cast_to: Ty<'tcx>,
) {
    if cast_from.is_fn() {
        let (from_snippet, _) = snippet_with_context(
            cx,
            cast_expr.span,
            expr.span.ctxt(),
            "..",
            &mut Applicability::Unspecified,
        );

        span_lint_and_then(
            cx,
            FN_TO_NUMERIC_CAST_ANY,
            expr.span,
            format!("casting function pointer `{from_snippet}` to `{cast_to}`"),
            |diag| {
                if !expr.span.from_expansion() && can_suggest_invoke(cx, cast_from, cast_to) {
                    let mut applicability = Applicability::MaybeIncorrect;
                    let mut sugg = Sugg::hir_with_context(cx, cast_expr, expr.span.ctxt(), "..", &mut applicability);
                    if matches!(
                        cast_expr.peel_drop_temps().kind,
                        ExprKind::Field(..) | ExprKind::Match(_, _, MatchSource::AwaitDesugar)
                    ) && cast_expr.span.ctxt() == expr.span.ctxt()
                        && let Sugg::NonParen(s) = sugg
                    {
                        sugg = Sugg::MaybeParen(s);
                    }

                    diag.span_suggestion_verbose(
                        expr.span,
                        "did you mean to invoke the function?",
                        format!("{}() as {cast_to}", sugg.maybe_paren()),
                        applicability,
                    );
                }
            },
        );
    }
}
