use clippy_utils::diagnostics::{span_lint, span_lint_and_then};
use clippy_utils::rinterval;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;

use super::{CAST_SIGN_LOSS, utils};

pub(super) fn check<'cx>(
    cx: &LateContext<'cx>,
    i_cx: &mut rinterval::IntervalCtxt<'_, 'cx>,
    expr: &Expr<'cx>,
    cast_op: &Expr<'cx>,
    cast_from: Ty<'cx>,
    cast_to: Ty<'_>,
) {
    // the to type has the be an unsigned integer type
    if !cast_to.is_integral() || cast_to.is_signed() {
        return;
    }

    // floating-point values can hold negative numbers that will all map to 0
    if cast_from.is_floating_point() {
        span_lint(
            cx,
            CAST_SIGN_LOSS,
            expr.span,
            format!("casting `{cast_from}` to `{cast_to}` may lose the sign of the value"),
        );
        return;
    }

    // Lastly, casting from signed integers to unsigned integers should only be
    // reported if the signed integer expression can actually contain negative
    // values.
    if cast_from.is_integral() && cast_from.is_signed() {
        if let Some(from_interval) = i_cx.eval(cast_op)
            && from_interval.ty.is_signed()
            && from_interval.contains_negative()
        {
            span_lint_and_then(
                cx,
                CAST_SIGN_LOSS,
                expr.span,
                format!("casting `{cast_from}` to `{cast_to}` may lose the sign of the value"),
                |diag| {
                    if !from_interval.is_full() {
                        diag.note(utils::format_cast_operand(from_interval));
                    }
                },
            );
        }
    }
}
