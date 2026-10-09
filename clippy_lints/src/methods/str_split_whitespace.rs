use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeResPath as _;
use clippy_utils::{peel_blocks, sym};
use rustc_ast::ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{BinOpKind, Closure, Expr, ExprKind, Pat, PatKind, PrimTy, UnOp};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol};

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
        // `!s.is_empty()`
        ExprKind::Unary(UnOp::Not, inner) => is_call_on_param(pat, inner, sym::is_empty),
        // `s.len() > 0`, `*s != ""`
        ExprKind::Binary(op, lhs, rhs) => is_non_empty_cmp(pat, op.node, lhs, rhs),
        _ => false,
    }
}

/// If `expr` is `x.name()` or `str::name(x)`, returns `name` and `x`.
fn method_or_path_call<'a>(expr: &'a Expr<'a>) -> Option<(Symbol, &'a Expr<'a>)> {
    match expr.kind {
        ExprKind::MethodCall(seg, recv, [], _) => Some((seg.ident.name, recv)),
        ExprKind::Call(callee, [arg])
            if let Some((ty, seg)) = callee.opt_ty_rel_path()
                && matches!(ty.basic_res(), Res::PrimTy(PrimTy::Str)) =>
        {
            Some((seg.ident.name, arg))
        },
        _ => None,
    }
}

/// Whether `expr` is `s.method()`, where `s` is the (possibly trimmed) closure parameter.
fn is_call_on_param(pat: &Pat<'_>, expr: &Expr<'_>, method: Symbol) -> bool {
    method_or_path_call(expr).is_some_and(|(name, inner)| name == method && is_closure_param(pat, peel_trims(inner)))
}

/// Strips any `trim`, `trim_start` and `trim_end` calls off `expr`.
fn peel_trims<'a>(expr: &'a Expr<'a>) -> &'a Expr<'a> {
    if let Some((name, inner)) = method_or_path_call(expr)
        && matches!(name, sym::trim | sym::trim_end | sym::trim_start)
    {
        peel_trims(inner)
    } else {
        expr
    }
}

/// Returns the operator that keeps the comparison's meaning when its operands are swapped.
fn swap_operator(op: BinOpKind) -> BinOpKind {
    match op {
        BinOpKind::Ge => BinOpKind::Le,
        BinOpKind::Gt => BinOpKind::Lt,
        BinOpKind::Le => BinOpKind::Ge,
        BinOpKind::Lt => BinOpKind::Gt,
        _ => op,
    }
}

/// Whether `lhs op rhs` holds only for a non-empty closure parameter.
fn is_non_empty_cmp(pat: &Pat<'_>, op: BinOpKind, lhs: &Expr<'_>, rhs: &Expr<'_>) -> bool {
    // Keep the literal on the right, so that `0 < s.len()` is handled as `s.len() > 0`.
    let (op, lhs, rhs) = if let ExprKind::Lit(_) = lhs.peel_borrows().kind {
        (swap_operator(op), rhs, lhs)
    } else {
        (op, lhs, rhs)
    };

    if let ExprKind::Lit(lit) = rhs.peel_borrows().kind {
        match lit.node {
            LitKind::Int(Pu128(value), _) => {
                matches!((op, value), (BinOpKind::Gt | BinOpKind::Ne, 0) | (BinOpKind::Ge, 1))
                    && is_call_on_param(pat, lhs, sym::len)
            },
            LitKind::Str(s, _) if s.as_str().is_empty() && op == BinOpKind::Ne => {
                let lhs = if let ExprKind::Unary(UnOp::Deref, inner) = lhs.kind {
                    inner
                } else {
                    lhs
                };
                is_closure_param(pat, peel_trims(lhs))
            },
            _ => false,
        }
    } else {
        false
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
