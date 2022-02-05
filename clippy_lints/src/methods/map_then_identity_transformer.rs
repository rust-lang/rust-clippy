use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::path_to_local_id;
use clippy_utils::visitors::expr_visitor;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{BodyId, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;
use rustc_span::{MultiSpan, Span};

use super::MAP_THEN_IDENTITY_TRANSFORMER;

pub(super) fn check<'tcx>(
    cx: &LateContext<'_>,
    meth1_span: Span,
    meth1_name: &str,
    meth1_clos: &'tcx Expr<'_>,
    meth2_name: &str,
    meth2_clos: &'tcx Expr<'_>,
) {
    if_chain!(
        if let ExprKind::Closure(_, _, meth1_clos_body_id, _, _) = &meth1_clos.kind;
        if let ExprKind::Closure(_, _, meth2_clos_body_id, _, _) = &meth2_clos.kind;
        if let Some(_) = refd_param_span(cx, meth1_clos_body_id);
        if let Some(refd_param_span) = refd_param_span(cx, meth2_clos_body_id);
        then {
            let meth1_clos_val = &cx.tcx.hir().body(*meth1_clos_body_id).value;

            span_lint_and_then(
                cx,
                MAP_THEN_IDENTITY_TRANSFORMER,
                MultiSpan::from_span(meth1_span),
                &format!("this `{meth1_name}` can be collapsed into the `{meth2_name}`"),
                |diag| {
                    let mut help_span = MultiSpan::from_spans(vec![meth1_clos_val.span, refd_param_span]);
                    help_span.push_span_label(refd_param_span, "replace this variable".into());
                    help_span.push_span_label(meth1_clos_val.span, "with this expression".into());
                    diag.span_help(help_span, &format!("these `{meth1_name}` and `{meth2_name}` can be merged into a single `{meth2_name}`"));
                },
            );
        }
    )
}

// On a given closure `|.., x| y`, checks if `x` is referenced just exactly once in `y` and returns
// the span of `x`
fn refd_param_span<'tcx>(cx: &LateContext<'tcx>, clos_body_id: &BodyId) -> Option<Span> {
    if_chain! {
        let clos_body = cx.tcx.hir().body(*clos_body_id);
        if let [.., clos_param] = clos_body.params;
        if let PatKind::Binding(_, clos_param_id, _, _) = clos_param.pat.kind;
        let clos_val = &clos_body.value;
        then {
            let mut count = 0;
            let mut span = None;
            expr_visitor(cx, |expr| {
                if path_to_local_id(expr, clos_param_id) {
                    count += 1;
                    span = Some(expr.span);
                }
                count <= 1
            })
            .visit_expr(clos_val);

            if count == 1 {
                span
            } else {
                None
            }
        } else {
            None
        }
    }
}
