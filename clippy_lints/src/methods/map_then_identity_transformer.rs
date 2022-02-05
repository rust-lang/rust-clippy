use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::path_to_local_id;
use clippy_utils::visitors::expr_visitor;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{BodyId, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;

use super::MAP_THEN_IDENTITY_TRANSFORMER;

pub(super) fn check<'tcx>(
    cx: &LateContext<'_>,
    expr: &'tcx Expr<'_>,
    map_name: &str,
    map_arg: &'tcx Expr<'_>,
    all_name: &str,
    all_arg: &'tcx Expr<'_>,
) {
    if_chain!(
        if let ExprKind::Closure(_, _, map_clos_body_id, map_arg_span, _) = &map_arg.kind;
        if let ExprKind::Closure(_, _, all_clos_body_id, all_arg_span, _) = &all_arg.kind;
        then {
            dbg!(param_refd_once(cx, map_clos_body_id));
            dbg!(param_refd_once(cx, all_clos_body_id));


            span_lint_and_help(
                cx,
                MAP_THEN_IDENTITY_TRANSFORMER,
                expr.span,
                &format!("map_all"),
                None,
                &format!("map_all"),
            );
        }
    )
}

fn param_refd_once<'tcx>(cx: &LateContext<'tcx>, clos_body_id: &BodyId) -> bool {
    if_chain! {
        let clos_body = cx.tcx.hir().body(*clos_body_id);
        if let [clos_param] = clos_body.params;
        if let PatKind::Binding(_, clos_param_id, _, _) = clos_param.pat.kind;
        let clos_val = &clos_body.value;
        then {
            let mut count = 0;
            expr_visitor(cx, |expr| {
                if path_to_local_id(expr, clos_param_id) {
                    count += 1;
                }
                count <= 1
            })
            .visit_expr(clos_val);

            count == 1
        } else {
            false
        }
    }
}
