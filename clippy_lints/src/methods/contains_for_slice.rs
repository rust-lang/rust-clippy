use clippy_utils::diagnostics::span_lint;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self};

use super::{CONTAINS_FOR_SLICE, method_call};

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if !expr.span.from_expansion()
    // any()
    && let Some((name, recv, args, _, _)) = method_call(expr)
    && name == "any"
    // check if the inner closure is a equality check
    && args.len() == 1
    && let ExprKind::Closure(closure) = args[0].kind
    && let body = cx.tcx.hir().body(closure.body)
    && let ExprKind::Binary(op, _, _) = body.value.kind
    && op.node == rustc_ast::ast::BinOpKind::Eq
    // iter()
    && let Some((name, recv, _, _, _)) = method_call(recv)
    && name == "iter"
    {
        // check if the receiver is a u8/i8 slice
        let ref_type = cx.typeck_results().expr_ty(recv);

        match ref_type.kind() {
            ty::Ref(_, inner_type, _)
                if inner_type.is_slice()
                    && let ty::Slice(slice_type) = inner_type.kind()
                    && (slice_type.to_string() == "u8" || slice_type.to_string() == "i8") =>
            {
                span_lint(
                    cx,
                    CONTAINS_FOR_SLICE,
                    expr.span,
                    "using `contains()` instead of `iter().any()` on u8/i8 slices is more efficient",
                );
            },
            _ => {},
        }
    }
}
