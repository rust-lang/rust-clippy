use clippy_utils::consts::ConstEvalCtxt;
use clippy_utils::diagnostics::span_lint_and_note;
use clippy_utils::is_from_proc_macro;
use clippy_utils::res::MaybeDef as _;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::MAP_OR_SAME_CONSTANT;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    recv: &'tcx Expr<'tcx>,
    def: &'tcx Expr<'tcx>,
    map: &'tcx Expr<'tcx>,
    is_map_or_else: bool,
) {
    if expr.span.from_expansion() || is_from_proc_macro(cx, expr) {
        return;
    }

    let recv_ty = cx.typeck_results().expr_ty_adjusted(recv);
    let Some(type_sym) = recv_ty.opt_diag_name(cx) else {
        return;
    };
    if !matches!(type_sym, sym::Option | sym::Result) {
        return;
    }

    let const_eval = ConstEvalCtxt::new(cx);

    let def_const = if is_map_or_else {
        if let ExprKind::Closure(closure) = def.kind {
            let body = cx.tcx.hir_body(closure.body);
            const_eval.eval(body.value.peel_blocks())
        } else {
            None
        }
    } else {
        const_eval.eval(def)
    };

    let Some(def_const) = def_const else {
        return;
    };

    let map_const = if let ExprKind::Closure(closure) = map.kind {
        let body = cx.tcx.hir_body(closure.body);
        const_eval.eval(body.value.peel_blocks())
    } else {
        None
    };

    if let Some(map_const) = map_const
        && def_const == map_const
    {
        let method_name = if is_map_or_else { "map_or_else" } else { "map_or" };
        span_lint_and_note(
            cx,
            MAP_OR_SAME_CONSTANT,
            expr.span,
            format!("both branches of `{method_name}` return the same constant value"),
            None,
            format!("this `{method_name}` always evaluates to the same value, regardless of the `{type_sym}` variant"),
        );
    }
}
