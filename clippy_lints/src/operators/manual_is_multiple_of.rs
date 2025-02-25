use clippy_utils::consts::{integer_const, is_zero_integer_const};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::SpanRangeExt;
use clippy_utils::sugg::Sugg;
use rustc_ast::BinOpKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;

use super::MANUAL_IS_MULTIPLE_OF;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'_>,
    op: BinOpKind,
    lhs: &'tcx Expr<'tcx>,
    rhs: &'tcx Expr<'tcx>,
    min_and_mask_size: u8,
    msrv: Msrv,
) {
    if msrv.meets(cx, msrvs::UNSIGNED_IS_MULTIPLE_OF)
        && let Some(operand) = uint_compare_to_zero(cx, op, lhs, rhs)
        && let ExprKind::Binary(lhs_op, lhs_left, lhs_right) = operand.kind
    {
        let mut app = Applicability::MachineApplicable;
        let (dividend, divisor) = if lhs_op.node == BinOpKind::Rem {
            (
                lhs_left,
                Sugg::hir_with_applicability(cx, lhs_right, "_", &mut app).into_string(),
            )
        } else if lhs_op.node == BinOpKind::BitAnd {
            let min_divisor = 1u128 << min_and_mask_size;
            if let Some(divisor) = is_all_ones(cx, lhs_right, min_divisor, &mut app) {
                (lhs_left, divisor)
            } else if let Some(divisor) = is_all_ones(cx, lhs_left, min_divisor, &mut app) {
                (lhs_right, divisor)
            } else {
                return;
            }
        } else {
            return;
        };
        span_lint_and_sugg(
            cx,
            MANUAL_IS_MULTIPLE_OF,
            expr.span,
            "manual implementation of `.is_multiple_of()`",
            "replace with",
            format!(
                "{}{}.is_multiple_of({divisor})",
                if op == BinOpKind::Eq { "" } else { "!" },
                Sugg::hir_with_applicability(cx, dividend, "_", &mut app).maybe_paren()
            ),
            app,
        );
    }
}

// If we have a `x == 0`, `x != 0` or `x > 0` (or the reverted ones), return the non-zero operand
fn uint_compare_to_zero<'tcx>(
    cx: &LateContext<'tcx>,
    op: BinOpKind,
    lhs: &'tcx Expr<'tcx>,
    rhs: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    let operand = if matches!(lhs.kind, ExprKind::Binary(..))
        && matches!(op, BinOpKind::Eq | BinOpKind::Ne | BinOpKind::Gt)
        && is_zero_integer_const(cx, rhs)
    {
        lhs
    } else if matches!(rhs.kind, ExprKind::Binary(..))
        && matches!(op, BinOpKind::Eq | BinOpKind::Ne | BinOpKind::Lt)
        && is_zero_integer_const(cx, lhs)
    {
        rhs
    } else {
        return None;
    };

    matches!(cx.typeck_results().expr_ty_adjusted(operand).kind(), ty::Uint(_)).then_some(operand)
}

/// If `expr` is provably made of all ones, return the representation of `expr+1` if it is no
/// smaller than `min_divisor`. This will catch expressions of the following forms:
/// - `(1 << A) - 1` where `A` is a constant
/// - integer literals — if it uses hexadecimal, the return value will as well
///
/// The function will not attempt to evaluate non-literal constant expressions, as those may depend
/// on conditional compilation.
fn is_all_ones<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    min_divisor: u128,
    app: &mut Applicability,
) -> Option<String> {
    if let ExprKind::Binary(op, lhs, rhs) = expr.kind
        && op.node == BinOpKind::Sub
        && let ExprKind::Binary(op, lhs_left, lhs_right) = lhs.kind
        && op.node == BinOpKind::Shl
        && let Some(1) = integer_const(cx, lhs_left)
        && let Some(1) = integer_const(cx, rhs)
        && integer_const(cx, lhs_right).is_none_or(|v| 1 << v >= min_divisor)
    {
        Some(Sugg::hir_with_applicability(cx, lhs, "_", app).to_string())
    } else if let Some(value) = integer_const(cx, expr)
        && let Some(inc_value) = value.checked_add(1)
        && inc_value.is_power_of_two()
    {
        let repr = if expr.span.check_source_text(cx, |s| s.starts_with("0x")) {
            format!("{inc_value:#x}")
        } else {
            inc_value.to_string()
        };
        (inc_value >= min_divisor).then_some(repr)
    } else {
        None
    }
}
