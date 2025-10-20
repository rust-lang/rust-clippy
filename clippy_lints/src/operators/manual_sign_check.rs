use clippy_utils::consts::{ConstEvalCtxt, Constant};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_applicability;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr};
use rustc_lint::LateContext;

use super::MANUAL_SIGN_CHECK;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, op: BinOpKind, left: &'tcx Expr<'_>, right: &'tcx Expr<'_>) {
    let (float_expr, direction) = if is_zero(cx, left) && is_float(cx, right) {
        (right, Side::Right)
    } else if is_zero(cx, right) && is_float(cx, left) {
        (left, Side::Left)
    } else {
        return;
    };

    let mut applicability = Applicability::MachineApplicable;
    let float_snippet = snippet_with_applicability(cx, float_expr.span, "..", &mut applicability);
    let (method, negate) = match (direction, op) {
        (Side::Left, BinOpKind::Lt) | (Side::Right, BinOpKind::Gt) => ("is_sign_negative", false),
        (Side::Left, BinOpKind::Le) | (Side::Right, BinOpKind::Ge) => ("is_sign_positive", true),
        (Side::Left, BinOpKind::Gt) | (Side::Right, BinOpKind::Lt) => ("is_sign_positive", false),
        (Side::Left, BinOpKind::Ge) | (Side::Right, BinOpKind::Le) => ("is_sign_negative", true),
        _ => return,
    };

    let suggestion = if negate {
        format!("!{float_snippet}.{method}()")
    } else {
        format!("{float_snippet}.{method}()")
    };

    span_lint_and_sugg(
        cx,
        MANUAL_SIGN_CHECK,
        float_expr.span.to(right.span),
        "checking the sign of a floating point number by comparing it to zero",
        format!("consider using `{method}` for clarity and performance"),
        suggestion,
        applicability,
    );
}

enum Side {
    Left,
    Right,
}

fn is_zero(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ecx = ConstEvalCtxt::new(cx);
    let Some(constant) = ecx.eval(expr) else {
        return false;
    };

    match constant {
        // FIXME(f16_f128): add when equality check is available on all platforms
        Constant::F32(f) => f == 0.0,
        Constant::F64(f) => f == 0.0,
        _ => false,
    }
}

fn is_float(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.typeck_results().expr_ty(expr).is_floating_point()
}
