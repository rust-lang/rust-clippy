use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::VecArgs;
use clippy_utils::macros::matching_root_macro_call;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet;
use clippy_utils::{local_is_initialized, sym};
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{Expr, ExprKind, QPath, TyKind, UnOp};
use rustc_lint::LateContext;

use super::NEW_INSTEAD_OF_CLEAR;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::Assign(lhs, rhs, _) = expr.kind
        && !expr.span.from_expansion()
    {
        check_collection_new(cx, expr, lhs, rhs);
        check_vec_macro(cx, expr, lhs, rhs);
    }
}

fn check_collection_new(cx: &LateContext<'_>, expr: &Expr<'_>, lhs: &Expr<'_>, rhs: &Expr<'_>) {
    // skip macro expansions (e.g. vec![] is handled separately)
    if !rhs.span.from_expansion()
        && let ExprKind::Call(func, []) = rhs.kind
        && let ExprKind::Path(QPath::TypeRelative(ty, method)) = func.kind
        && method.ident.name == sym::new
        // turbofish on the type (e.g. `Vec::<i32>::new()`) changes type inference,
        // so replacing with `.clear()` could be wrong
        && !has_type_args(ty)
    {
        let rhs_ty = cx.typeck_results().node_type(ty.hir_id);
        if matches!(
            rhs_ty.peel_refs().opt_diag_name(cx),
            Some(
                sym::Vec
                    | sym::HashMap
                    | sym::HashSet
                    | sym::VecDeque
                    | sym::BTreeMap
                    | sym::BTreeSet
                    | sym::BinaryHeap
                    | sym::LinkedList
            )
        ) && let Some(sugg) = local_snippet(cx, lhs)
        {
            emit_lint(cx, expr, &sugg);
        }
    }
}

fn check_vec_macro(cx: &LateContext<'_>, expr: &Expr<'_>, lhs: &Expr<'_>, rhs: &Expr<'_>) {
    if matching_root_macro_call(cx, rhs.span, sym::vec_macro).is_some()
        && matches!(VecArgs::hir(cx, rhs), Some(VecArgs::Vec([])))
        && let Some(sugg) = local_snippet(cx, lhs)
    {
        emit_lint(cx, expr, &sugg);
    }
}

fn emit_lint(cx: &LateContext<'_>, expr: &Expr<'_>, sugg: &str) {
    span_lint_and_then(
        cx,
        NEW_INSTEAD_OF_CLEAR,
        expr.span,
        "assigning a new empty collection",
        |diag| {
            diag.span_suggestion(
                expr.span,
                "consider using `.clear()` instead",
                format!("{sugg}.clear()"),
                Applicability::MaybeIncorrect,
            );
            diag.note("`.clear()` retains the allocated memory for reuse");
        },
    );
}

fn has_type_args(ty: &rustc_hir::Ty<'_>) -> bool {
    if let TyKind::Path(QPath::Resolved(_, path)) = ty.kind {
        return path.segments.last().is_some_and(|seg| seg.args.is_some());
    }
    false
}

/// Returns the source snippet for `expr` if it refers to an initialized local variable,
/// peeling any leading dereferences first.
fn local_snippet(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<String> {
    let inner = peel_derefs(expr);
    if let ExprKind::Path(QPath::Resolved(None, path)) = inner.kind
        && let Res::Local(hir_id) = path.res
        && local_is_initialized(cx, hir_id)
    {
        Some(snippet(cx, inner.span, "..").into_owned())
    } else {
        None
    }
}

fn peel_derefs<'tcx>(mut expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    while let ExprKind::Unary(UnOp::Deref, inner) = expr.kind {
        expr = inner;
    }
    expr
}
