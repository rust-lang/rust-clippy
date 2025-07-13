use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{is_expr_untyped_identity_function, is_mutable, is_trait_method, path_to_local};
use rustc_errors::Applicability;
use rustc_hir::{self as hir, ExprKind, Node, PatKind};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol, sym};

use super::MAP_IDENTITY;

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &hir::Expr<'_>,
    caller: &hir::Expr<'_>,
    map_arg: &hir::Expr<'_>,
    name: Symbol,
    _map_span: Span,
) {
    let caller_ty = cx.typeck_results().expr_ty(caller);

    if (is_trait_method(cx, expr, sym::Iterator)
        || is_type_diagnostic_item(cx, caller_ty, sym::Result)
        || is_type_diagnostic_item(cx, caller_ty, sym::Option))
        && is_expr_untyped_identity_function(cx, map_arg)
        && let Some(call_span) = expr.span.trim_start(caller.span)
    {
        let mut sugg = vec![(call_span, String::new())];
        let mut apply = true;
        if !is_mutable(cx, caller) {
            if let Some(hir_id) = path_to_local(caller)
                && let Node::Pat(pat) = cx.tcx.hir_node(hir_id)
                && let PatKind::Binding(_, _, ident, _) = pat.kind
            {
                sugg.push((ident.span.shrink_to_lo(), String::from("mut ")));
            } else {
                // If we can't make the binding mutable, make the suggestion `Unspecified` to prevent it from being
                // automatically applied, and add a complementary help message.
                apply = false;
            }
        }

        let method_requiring_mut = if let Node::Expr(expr) = cx.tcx.parent_hir_node(expr.hir_id)
            && let ExprKind::MethodCall(method, ..) = expr.kind
        {
            Some(method.ident)
        } else {
            None
        };

        span_lint_and_then(
            cx,
            MAP_IDENTITY,
            call_span,
            "unnecessary map of the identity function",
            |diag| {
                diag.multipart_suggestion(
                    format!("remove the call to `{name}`"),
                    sugg,
                    if apply {
                        Applicability::MachineApplicable
                    } else {
                        Applicability::Unspecified
                    },
                );
                if !apply {
                    if let Some(method_requiring_mut) = method_requiring_mut {
                        diag.span_note(
                            caller.span,
                            format!("this must be made mutable to use `{method_requiring_mut}`"),
                        );
                    } else {
                        diag.span_note(caller.span, "this must be made mutable".to_string());
                    }
                }
            },
        );
    }
}
