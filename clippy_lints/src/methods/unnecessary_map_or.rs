use clippy_config::msrvs::{self, Msrv};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::eager_or_lazy::switch_to_eager_eval;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::visitors::is_local_used;
use clippy_utils::{get_parent_expr, path_to_local_id};
use rustc_ast::util::parser::AssocOp;
use rustc_ast::LitKind::Bool;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::UNNECESSARY_MAP_OR;

// Only checking map_or for now
pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    recv: &Expr<'_>,
    def: &Expr<'_>,
    map: &Expr<'_>,
    msrv: &Msrv,
) {
    if let ExprKind::Lit(def_kind) = def.kind
        && let typeck_results = cx.typeck_results()
        && let recv_ty = typeck_results.expr_ty(recv)
        && (is_type_diagnostic_item(cx, recv_ty, sym::Option) || is_type_diagnostic_item(cx, recv_ty, sym::Result))
        && let Bool(def_bool) = def_kind.node
    {
        let (sugg, method) = if let ExprKind::Closure(map_closure) = map.kind
        && let closure_body = cx.tcx.hir().body(map_closure.body)
        && let closure_body_value = closure_body.value.peel_blocks()
        && let ExprKind::Binary(op, l, r) = closure_body_value.kind
        && let Some(param) = closure_body.params.first()
        && let PatKind::Binding(_, hir_id, _, _) = param.pat.kind
        // checking that map_or is either:
        // .map_or(false, |x| x == y) OR .map_or(true, |x| x != y)
        && ((BinOpKind::Eq == op.node && !def_bool) || (BinOpKind::Ne == op.node && def_bool))
        && let non_binding_location = if path_to_local_id(l, hir_id) { r } else { l }
        && switch_to_eager_eval(cx, non_binding_location)
        // xor, because if its both then thats a strange edge case and
        // we can just ignore it, since by default clippy will error on this
        && (path_to_local_id(l, hir_id) ^ path_to_local_id(r, hir_id))
        && !is_local_used(cx, non_binding_location, hir_id)
        && typeck_results.expr_ty(l) == typeck_results.expr_ty(r)
        {
            let wrap = if is_type_diagnostic_item(cx, recv_ty, sym::Option) {
                "Some"
            } else {
                "Ok"
            };
            let comparator = if BinOpKind::Eq == op.node { "==" } else { "!=" };

            // we may need to add parens around the suggestion
            // in case the parent expression has additional method calls,
            // since for example `Some(5).map_or(false, |x| x == 5).then(|| 1)`
            // being converted to `Some(5) == Some(5).then(|| 1)` isnt
            // the same thing
            let should_add_parens = get_parent_expr(cx, expr)
                .is_some_and(|expr| expr.precedence().order() > i8::try_from(AssocOp::Equal.precedence()).unwrap_or(0));
            (
                format!(
                    "{}{} {} {}({}){}",
                    if should_add_parens { "(" } else { "" },
                    snippet(cx, recv.span, ".."),
                    comparator,
                    wrap,
                    snippet(cx, non_binding_location.span.source_callsite(), ".."),
                    if should_add_parens { ")" } else { "" }
                ),
                "standard comparison",
            )
        } else if !def_bool && msrv.meets(msrvs::OPTION_RESULT_IS_VARIANT_AND) {
            let sugg = if is_type_diagnostic_item(cx, recv_ty, sym::Option) {
                "is_some_and"
            } else {
                "is_ok_and"
            };

            (
                format!(
                    "{}.{}({})",
                    snippet(cx, recv.span.source_callsite(), ".."),
                    sugg,
                    snippet(cx, map.span.source_callsite(), "..")
                ),
                sugg,
            )
        } else {
            return;
        };

        span_lint_and_sugg(
            cx,
            UNNECESSARY_MAP_OR,
            expr.span,
            &format!("`map_or` is redundant, use {method} instead"),
            "try",
            sugg,
            Applicability::MachineApplicable,
        );
    }
}
