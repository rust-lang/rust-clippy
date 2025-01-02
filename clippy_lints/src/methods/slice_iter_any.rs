use clippy_utils::diagnostics::span_lint;
use clippy_utils::msrvs::Msrv;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self};
use rustc_session::RustcVersion;
use rustc_span::source_map::Spanned;

use super::{SLICE_ITER_ANY, method_call};

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, msrv: &Msrv) {
    if !expr.span.from_expansion()
    // any()
    && let Some((name, recv, args, _, _)) = method_call(expr)
    && name == "any"
    // check if `iter().any()` can be replaced with `contains()`
    && args.len() == 1
    && let ExprKind::Closure(closure) = args[0].kind
    && let body = cx.tcx.hir().body(closure.body)
    && let ExprKind::Binary(op, lhs, rhs) = body.value.kind
    && can_replace_with_contains(op, lhs, rhs)
    // iter()
    && let Some((name, recv, _, _, _)) = method_call(recv)
    && name == "iter"
    {
        let ref_type = cx.typeck_results().expr_ty_adjusted(recv);

        match ref_type.kind() {
            ty::Ref(_, inner_type, _) if inner_type.is_slice() => {
                // check if the receiver is a numeric slice
                if let ty::Slice(slice_type) = inner_type.kind()
                    && ((slice_type.to_string() == "u8" || slice_type.to_string() == "i8")
                        || (slice_type.is_numeric()
                            && msrv.meets(RustcVersion {
                                major: 1,
                                minor: 84,
                                patch: 0,
                            })))
                {
                    span_lint(
                        cx,
                        SLICE_ITER_ANY,
                        expr.span,
                        "using `contains()` instead of `iter().any()` is more efficient",
                    );
                }
            },
            _ => {},
        }
    }
}

fn can_replace_with_contains(op: Spanned<BinOpKind>, lhs: &Expr<'_>, rhs: &Expr<'_>) -> bool {
    matches!(
        (op.node, &lhs.kind, &rhs.kind),
        (
            BinOpKind::Eq,
            ExprKind::Path(_) | ExprKind::Unary(_, _),
            ExprKind::Lit(_) | ExprKind::Path(_)
        ) | (
            BinOpKind::Eq,
            ExprKind::Lit(_),
            ExprKind::Path(_) | ExprKind::Unary(_, _)
        )
    )
}
