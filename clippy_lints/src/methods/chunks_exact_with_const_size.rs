use super::CHUNKS_EXACT_WITH_CONST_SIZE;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::ForLoop;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_context;
use clippy_utils::visitors::is_const_evaluatable;
use clippy_utils::{expr_use_ctxt, get_parent_expr, sym};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Node, PatKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{Span, Symbol};

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    recv: &'tcx Expr<'tcx>,
    arg: &'tcx Expr<'tcx>,
    expr: &'tcx Expr<'tcx>,
    call_span: Span,
    method_name: Symbol,
    msrv: Msrv,
) {
    let recv_ty = cx.typeck_results().expr_ty_adjusted(recv);
    if !matches!(recv_ty.kind(), ty::Ref(_, inner, _) if inner.is_slice()) {
        return;
    }

    if is_const_evaluatable(cx, arg) {
        if !msrv.meets(cx, msrvs::AS_CHUNKS) {
            return;
        }

        let suggestion_method = if method_name == sym::chunks_exact_mut {
            "as_chunks_mut"
        } else {
            "as_chunks"
        };

        let mut applicability = Applicability::MachineApplicable;
        let arg_str = snippet_with_context(cx, arg.span, expr.span.ctxt(), "_", &mut applicability).0;

        let as_chunks = format_args!("{suggestion_method}::<{arg_str}>()");

        span_lint_and_then(
            cx,
            CHUNKS_EXACT_WITH_CONST_SIZE,
            call_span,
            format!("using `{method_name}` with a constant chunk size"),
            |diag| {
                let use_ctxt = expr_use_ctxt(cx, expr);

                let in_for_loop = {
                    let mut cur_expr = expr;
                    loop {
                        if let Some(parent_expr) = get_parent_expr(cx, cur_expr) {
                            if let Some(for_loop) = ForLoop::hir(parent_expr)
                                && for_loop.arg.hir_id == expr.hir_id
                            {
                                break true;
                            }
                            cur_expr = parent_expr;
                        } else {
                            break false;
                        }
                    }
                };

                let is_iterator_method = if let Node::Expr(parent_expr) = use_ctxt.node
                    && let ExprKind::MethodCall(_, receiver, _, _) = parent_expr.kind
                    && receiver.hir_id == use_ctxt.child_id
                    && let Some(method_did) = cx.typeck_results().type_dependent_def_id(parent_expr.hir_id)
                    && let Some(trait_did) = cx.tcx.trait_of_assoc(method_did)
                {
                    matches!(
                        cx.tcx.get_diagnostic_name(trait_did),
                        Some(sym::Iterator | sym::IntoIterator)
                    )
                } else {
                    false
                };

                if in_for_loop {
                    diag.span_suggestion(
                        call_span,
                        "consider using `as_chunks` instead",
                        format!("{as_chunks}.0"),
                        applicability,
                    );
                } else if is_iterator_method {
                    diag.span_suggestion(
                        call_span,
                        "consider using `as_chunks` instead",
                        format!("{as_chunks}.0.iter()"),
                        applicability,
                    );
                } else {
                    diag.span_help(call_span, format!("consider using `{as_chunks}` instead"));

                    if let Node::LetStmt(let_stmt) = use_ctxt.node
                        && let PatKind::Binding(_, _, ident, _) = let_stmt.pat.kind
                    {
                        diag.note(format!(
                            "you can access the chunks using `{ident}.0.iter()`, and the remainder using `{ident}.1`"
                        ));
                    }
                }
            },
        );
    }
}
