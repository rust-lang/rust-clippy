use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::is_from_proc_macro;
use clippy_utils::res::MaybeDef as _;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::INVALID_NONZERO;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, func: &Expr<'tcx>, args: &[Expr<'tcx>]) {
    if let [arg] = args
        && let ExprKind::Path(QPath::TypeRelative(ty, segment)) = func.kind
        && segment.ident.name == sym::new
        && let nonzero_ty = cx.typeck_results().node_type(ty.hir_id)
        && nonzero_ty.is_diag_item(cx, sym::NonZero)
        && let Some(Constant::Int(0)) = ConstEvalCtxt::new(cx).eval(arg)
        && !is_from_proc_macro(cx, expr)
    {
        span_lint_and_sugg(
            cx,
            INVALID_NONZERO,
            expr.span,
            "a `NonZero` value crated from a `0` literal will always be `None`",
            "use",
            format!("None::<{nonzero_ty}>"),
            Applicability::MaybeIncorrect,
        );
    }
}
