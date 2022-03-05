use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::path_to_local_id;
use clippy_utils::visitors::expr_visitor;
use rustc_hir::intravisit::Visitor;
use rustc_hir::{BodyId, Expr, ExprKind, PatKind};
use rustc_lint::{LateContext, LintContext};
use rustc_span::{MultiSpan, Span};

use super::MAP_THEN_IDENTITY_TRANSFORMER;

pub(super) fn check<'tcx>(
    cx: &LateContext<'_>,
    map_span: Span,
    map_name: &str,
    map_param: &'tcx Expr<'_>,
    transformer_name: &str,
    // Use the last parameter of the transformer because the transfomer may be `fold(_, _)`
    transformer_last_param: &'tcx Expr<'_>,
) {
    match &transformer_last_param.kind {
        ExprKind::Closure(_, _, transformer_clos_body_id, _, _) => {
            match &map_param.kind {
                // map(Closure).<transformer>(Closure)
                ExprKind::Closure(_, _, map_clos_body_id, _, _) => {
                    let map_clos_val = &cx.tcx.hir().body(*map_clos_body_id).value;
                    if_chain!(
                        // checks if the body of the closure of the `map` is an one-line expression
                        if is_one_line(cx, map_clos_val.span);
                        // checks if the parameter of the closure of the transformer appears once in its body
                        if let Some(refd_param_span) = refd_param_span(cx, *transformer_clos_body_id);
                        then {
                            span_lint_and_then(
                                cx,
                                MAP_THEN_IDENTITY_TRANSFORMER,
                                MultiSpan::from_span(map_span),
                                &format!("this `{map_name}` can be collapsed into the `{transformer_name}`"),
                                |diag| {
                                    let mut help_span = MultiSpan::from_spans(vec![map_clos_val.span, refd_param_span]);
                                    help_span.push_span_label(refd_param_span, "replace this variable".into());
                                    help_span.push_span_label(map_clos_val.span, "with this expression".into());
                                    diag.span_help(help_span, &format!("these `{map_name}` and `{transformer_name}` can be merged into a single `{transformer_name}`"));
                                },
                            );
                        }

                    );
                },
                // map(Path).<transformer>(Closure)
                ExprKind::Path(_) => {
                    if_chain!(
                        // checks if the parameter of the `map` fits within one line
                        if is_one_line(cx, map_param.span);
                        // checks if the parameter of the closure of the transformer appears once in its body
                        if let Some(refd_param_span) = refd_param_span(cx, *transformer_clos_body_id);
                        then {
                            span_lint_and_then(
                                cx,
                                MAP_THEN_IDENTITY_TRANSFORMER,
                                MultiSpan::from_span(map_span),
                                &format!("this `{map_name}` can be collapsed into the `{transformer_name}`"),
                                |diag| {
                                    let mut help_span = MultiSpan::from_spans(vec![map_param.span, refd_param_span]);
                                    help_span.push_span_label(map_param.span, "apply this".into());
                                    help_span.push_span_label(refd_param_span, "to this variable".into());
                                    diag.span_help(help_span, &format!("these `{map_name}` and `{transformer_name}` can be merged into a single `{transformer_name}`"));
                                },
                            );
                        }

                    );
                },
                _ => (),
            }
        },
        // map(Path).<transformer>(Path) or map(Closure).<transformer>(Path)
        ExprKind::Path(_) => {
            if_chain!(
                // checks if the parameter of the `map` fits within one line
                if is_one_line(cx, map_param.span);
                then {
                    span_lint_and_then(
                        cx,
                        MAP_THEN_IDENTITY_TRANSFORMER,
                        MultiSpan::from_span(map_span),
                        &format!("this `{map_name}` can be collapsed into the `{transformer_name}`"),
                        |diag| {
                            let mut help_span = MultiSpan::from_spans(
                                vec![map_param.span, transformer_last_param.span]
                            );
                            help_span.push_span_label(map_param.span, format!("and use this in the `{transformer_name}`"));
                            help_span.push_span_label(transformer_last_param.span, "change this to a closure".into());
                            diag.span_help(help_span, &format!("these `{map_name}` and `{transformer_name}` can be merged into a single `{transformer_name}`"));
                        },
                    );
                }
            );
        },
        _ => (),
    }
}

// Given a closure `|.., x| y`, checks if `x` is referenced just exactly once in `y` and returns
// the span of `x` in `y`
fn refd_param_span(cx: &LateContext<'_>, clos_body_id: BodyId) -> Option<Span> {
    let clos_body = cx.tcx.hir().body(clos_body_id);
    if_chain! {
        if let [.., clos_param] = clos_body.params;
        if let PatKind::Binding(_, clos_param_id, _, _) = clos_param.pat.kind;
        then {
            let clos_val = &clos_body.value;
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

fn is_one_line(cx: &LateContext<'_>, span: Span) -> bool {
    let src = cx.sess().source_map();
    if let Ok(lo) = src.lookup_line(span.lo()) {
        if let Ok(hi) = src.lookup_line(span.hi()) {
            return hi.line == lo.line;
        }
    }
    false
}
