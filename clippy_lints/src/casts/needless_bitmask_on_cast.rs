use super::NEEDLESS_BITMASK_ON_CAST;
use clippy_utils::consts::ConstEvalCtxt;
use clippy_utils::consts::Constant::Int;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::expr_or_init;
use clippy_utils::source::snippet;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;

pub(super) fn check<'a>(cx: &LateContext<'a>, expr: &Expr<'_>, cast_expr: &Expr<'_>, _from_ty: Ty<'_>, to_ty: Ty<'a>) {
    let app = Applicability::MaybeIncorrect;
    // Checking that the cast is to an integer type and that the operation being performed is an And
    if to_ty.is_integral()
        && let size = to_ty.primitive_size(cx.tcx)
        && let ExprKind::Binary(op, left, right) = cast_expr.kind
        && op.node == BinOpKind::BitAnd
    {
        // For the sake of supporting commutativity
        let mask_and_value = [(left, right), (right, left)];
        for (mask_op, value_op) in mask_and_value {
            //Constant evaluation
            let operand_init = expr_or_init(cx, mask_op);
            let const_context = ConstEvalCtxt::new(cx);
            if let Some(const_mask) = const_context.eval(operand_init)
                && let Int(mask_value) = const_mask
                && (mask_value & size.unsigned_int_max()) == size.unsigned_int_max()
            {
                let value_snip = snippet(cx, value_op.span, "..");
                let suggestion = format!("{value_snip} as {to_ty}");
                span_lint_and_sugg(
                    cx,
                    NEEDLESS_BITMASK_ON_CAST,
                    expr.span,
                    "unnecessary bitmask on cast",
                    "the cast already truncates the input",
                    suggestion,
                    app,
                );
            }
        }
    }
}
