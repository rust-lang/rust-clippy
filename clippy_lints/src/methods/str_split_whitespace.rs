use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeResPath as _;
use clippy_utils::{peel_blocks, sym};
use rustc_ast::ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Closure, Expr, ExprKind, Pat, PatKind, UnOp};
use rustc_lint::LateContext;
use rustc_span::Span;

use crate::methods::method_call;

use super::STR_SPLIT_WHITESPACE;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    filter_recv: &'tcx Expr<'_>,
    filter_span: Span,
    filter_arg: &'tcx Expr<'_>,
) {
    // We're looking for `A.split(B).filter(C)`, where the adjusted type of `A` is `&str` (e.g. an
    // expression returning `String`), `B` is a literal single space, and `C` is a closure that
    // discards the empty substrings which splitting on a single space produced.
    if let ExprKind::MethodCall(split_seg, split_recv, [split_arg], split_span) = filter_recv.kind
        && split_seg.ident.name == sym::split
        && let ExprKind::Lit(split_lit) = split_arg.kind
        && matches!(split_lit.node, LitKind::Char(' ') | LitKind::Str(sym::SPACE, _))
        && cx.typeck_results().expr_ty_adjusted(split_recv).peel_refs().is_str()
        && let ExprKind::Closure(&Closure { body, .. }) = filter_arg.kind
        && let body = cx.tcx.hir_body(body)
        && let [closure_arg] = body.params
        && discards_empty(closure_arg.pat, peel_blocks(body.value))
    {
        span_lint_and_then(
            cx,
            STR_SPLIT_WHITESPACE,
            expr.span,
            "using `str.split().filter()` to discard empty substrings",
            |diag| {
                diag.span_suggestion_verbose(
                    split_span.to(filter_span),
                    "use `str.split_whitespace()` instead",
                    "split_whitespace()",
                    Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

/// Whether `expr`, the body of the `filter` closure, discards the empty substrings.
fn discards_empty(pat: &Pat<'_>, expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Unary(UnOp::Not, inner) => match method_call(inner) {
            // `|s| !s.is_empty()`
            Some((sym::is_empty, recv, [], _, _)) if is_closure_param(pat, recv) => true,
            // `|s| !s.trim().is_empty()`
            Some((sym::is_empty, recv, [], _, _))
                if matches!(method_call(recv), Some((sym::trim, trim_recv, [], _, _))
                    if is_closure_param(pat, trim_recv)) =>
            {
                true
            },
            // `|s| !str::is_empty(s)`
            None if matches!(inner.kind, ExprKind::Call(callee, [arg])
                if matches!(callee.opt_ty_rel_path(), Some((_, seg)) if seg.ident.name == sym::is_empty)
                    && is_closure_param(pat, arg)) =>
            {
                true
            },
            _ => false,
        },
        // `|s| s.len() > 0`
        ExprKind::Binary(op, lhs, rhs)
            if op.node == BinOpKind::Gt
                && matches!(rhs.kind, ExprKind::Lit(lit) if matches!(lit.node, LitKind::Int(Pu128(0), _)))
                && matches!(method_call(lhs), Some((sym::len, len_recv, [], _, _))
                    if is_closure_param(pat, len_recv)) =>
        {
            true
        },
        _ => false,
    }
}

/// Whether `expr` is a reference to the binding introduced by `pat`.
fn is_closure_param(pat: &Pat<'_>, expr: &Expr<'_>) -> bool {
    match pat.kind {
        PatKind::Binding(_, id, _, None) => expr.res_local_id() == Some(id),
        // `filter` hands the closure a `&&str`, so `|&s|` is written just as often as `|s|`.
        PatKind::Ref(pat, ..) => is_closure_param(pat, expr),
        _ => false,
    }
}
