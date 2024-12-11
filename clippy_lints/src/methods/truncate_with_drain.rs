use clippy_utils::consts::{ConstEvalCtxt, mir_to_const};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher;
use clippy_utils::source::snippet;
use clippy_utils::ty::{is_type_diagnostic_item, is_type_lang_item};
use rustc_ast::ast::RangeLimits;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, LangItem, Path, QPath};
use rustc_lint::LateContext;
use rustc_middle::mir::Const;
use rustc_middle::ty::{Adt, Ty, TypeckResults};
use rustc_span::Span;
use rustc_span::symbol::sym;

use super::TRUNCATE_WITH_DRAIN;

// Add `String` here when it is added to diagnostic items
const ACCEPTABLE_TYPES_WITH_ARG: [rustc_span::Symbol; 2] = [sym::Vec, sym::VecDeque];

pub fn is_range_open_ended<'a>(
    cx: &LateContext<'a>,
    range: higher::Range<'_>,
    ty: Ty<'a>,
    container_path: Option<&Path<'_>>,
) -> bool {
    let higher::Range { start, end, limits } = range;
    let start_is_none_or_min = start.map_or(true, |start| {
        if let Adt(_, subst) = ty.kind()
            && let bnd_ty = subst.type_at(0)
            && let Some(min_val) = bnd_ty.numeric_min_val(cx.tcx)
            && let Some(min_const) = mir_to_const(cx.tcx, Const::from_ty_const(min_val, bnd_ty, cx.tcx))
            && let Some(start_const) = ConstEvalCtxt::new(cx).eval(start)
        {
            start_const == min_const
        } else {
            false
        }
    });
    let end_is_none_or_max = end.map_or(true, |end| match limits {
        RangeLimits::Closed => {
            if let Adt(_, subst) = ty.kind()
                && let bnd_ty = subst.type_at(0)
                && let Some(max_val) = bnd_ty.numeric_max_val(cx.tcx)
                && let Some(max_const) = mir_to_const(cx.tcx, Const::from_ty_const(max_val, bnd_ty, cx.tcx))
                && let Some(end_const) = ConstEvalCtxt::new(cx).eval(end)
            {
                end_const == max_const
            } else {
                false
            }
        },
        RangeLimits::HalfOpen => {
            if let Some(container_path) = container_path
                && let ExprKind::MethodCall(name, self_arg, [], _) = end.kind
                && name.ident.name == sym::len
                && let ExprKind::Path(QPath::Resolved(None, path)) = self_arg.kind
            {
                container_path.res == path.res
            } else {
                false
            }
        },
    });
    !start_is_none_or_min && end_is_none_or_max
}

fn match_acceptable_type(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    typeck_results: &TypeckResults<'_>,
    types: &[rustc_span::Symbol],
) -> bool {
    let expr_ty = typeck_results.expr_ty(expr).peel_refs();
    types.iter().any(|&ty| is_type_diagnostic_item(cx, expr_ty, ty))
    // String type is a lang item but not a diagnostic item for now so we need a separate check
        || is_type_lang_item(cx, expr_ty, LangItem::String)
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, span: Span, arg: Option<&Expr<'_>>) {
    if let Some(arg) = arg {
        let typeck_results = cx.typeck_results();
        if match_acceptable_type(cx, recv, typeck_results, &ACCEPTABLE_TYPES_WITH_ARG)
            && let ExprKind::Path(QPath::Resolved(None, container_path)) = recv.kind
            && let Some(range) = higher::Range::hir(arg)
            && let higher::Range { start: Some(start), .. } = range
            && is_range_open_ended(cx, range, typeck_results.expr_ty(arg), Some(container_path))
            && let Some(adt) = typeck_results.expr_ty(recv).ty_adt_def()
            // Use `opt_item_name` while `String` is not a diagnostic item
            && let Some(ty_name) = cx.tcx.opt_item_name(adt.did())
        {
            span_lint_and_sugg(
                cx,
                TRUNCATE_WITH_DRAIN,
                span.with_hi(expr.span.hi()),
                format!("`drain` used to truncate a `{ty_name}`"),
                "try",
                format!("truncate({})", snippet(cx, start.span, "0")),
                Applicability::MachineApplicable,
            );
        }
    }
}
