use std::cmp::Ordering;

use super::UNNECESSARY_MIN;
use clippy_utils::diagnostics::span_lint_and_sugg;

use clippy_utils::consts::{constant, Constant};
use clippy_utils::source::snippet;
use clippy_utils::{clip, int_bits, unsext};
use hir::Expr;

use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;

use rustc_middle::ty::{self, IntTy};
use rustc_span::Span;

pub fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, recv: &'tcx Expr<'_>, arg: &'tcx Expr<'_>) {
    if both_are_constant(cx, expr, recv, arg) {
        return;
    }
    one_extrema(cx, expr, recv, arg);
}
fn lint(cx: &LateContext<'_>, expr: &Expr<'_>, sugg: Span, other: Span) {
    let msg = format!(
        "`{}` is never greater than `{}` and has therefore no effect",
        snippet(cx, sugg, "Not yet implemented"),
        snippet(cx, other, "Not yet implemented")
    );
    span_lint_and_sugg(
        cx,
        UNNECESSARY_MIN,
        expr.span,
        &msg,
        "try",
        snippet(cx, sugg, "Not yet implemented").to_string(),
        Applicability::MachineApplicable,
    );
}

fn try_to_eval<'tcx>(
    cx: &LateContext<'tcx>,
    recv: &'tcx Expr<'_>,
    arg: &'tcx Expr<'_>,
) -> (Option<Constant<'tcx>>, Option<Constant<'tcx>>) {
    (
        (constant(cx, cx.typeck_results(), recv)),
        (constant(cx, cx.typeck_results(), arg)),
    )
}
#[derive(Debug)]
enum Extrema {
    Minimum,
    Maximum,
}
fn detect_extrema<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) -> Option<Extrema> {
    let ty = cx.typeck_results().expr_ty(expr);

    let cv = constant(cx, cx.typeck_results(), expr)?;

    match (ty.kind(), cv) {
        (&ty::Uint(_), Constant::Int(0)) => Some(Extrema::Minimum),
        (&ty::Int(ity), Constant::Int(i)) if i == unsext(cx.tcx, i128::MIN >> (128 - int_bits(cx.tcx, ity)), ity) => {
            Some(Extrema::Minimum)
        },

        (&ty::Int(ity), Constant::Int(i)) if i == unsext(cx.tcx, i128::MAX >> (128 - int_bits(cx.tcx, ity)), ity) => {
            Some(Extrema::Maximum)
        },
        (&ty::Uint(uty), Constant::Int(i)) if i == clip(cx.tcx, u128::MAX, uty) => Some(Extrema::Maximum),

        _ => None,
    }
}
fn cmp_for_signed(a: u128, b: u128, cx: &LateContext<'_>, ty: IntTy) -> Ordering {
    let a_sign = Sign::from((a, cx, ty));
    let b_sign = Sign::from((b, cx, ty));
    // The Ordering of a signed integer interpreted as a unsigned integer is as follows:
    // -1       b1111...    uX::MAX
    // iX::MIN  b1000...
    // iX::MAX  b0111...
    // 0        b0000...    uX::MIN
    match (a_sign, b_sign) {
        (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => a.cmp(&b),
        (Sign::Positive, Sign::Negative) => Ordering::Greater,
        (Sign::Negative, Sign::Positive) => Ordering::Less,
    }
}
#[derive(Debug)]
enum Sign {
    Positive,
    Negative,
}
impl From<(u128, &LateContext<'_>, IntTy)> for Sign {
    fn from(value: (u128, &LateContext<'_>, IntTy)) -> Self {
        // The MSB decides whether the value has a negative sign in front of it or not
        // the value 0 is counting as positive (or as non-negative)
        let (value, cx, ity) = value;
        // shifting the MSB from a iX (i32, i64, etc) to the MSB from a i128
        let value = value << (128 - int_bits(cx.tcx, ity));
        let msb = (value.reverse_bits()) & 1_u128; // single out the MSB
        match msb {
            0 => Self::Positive,
            1 => Self::Negative,
            _ => unreachable!("Bit can only be 0 or 1"),
        }
    }
}
fn both_are_constant<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    recv: &'tcx Expr<'_>,
    arg: &'tcx Expr<'_>,
) -> bool {
    let ty = cx.typeck_results().expr_ty(recv);
    if let (Some(left), Some(right)) = try_to_eval(cx, recv, arg) {
        let ord = match (ty.kind(), left, right) {
            (ty::Int(ty), Constant::Int(left), Constant::Int(right)) => cmp_for_signed(left, right, cx, *ty),
            (ty::Uint(_), Constant::Int(left), Constant::Int(right)) => left.cmp(&right),
            _ => return false,
        };

        let (sugg, other) = match ord {
            Ordering::Less => (recv.span, arg.span),
            Ordering::Equal | Ordering::Greater => (arg.span, recv.span),
        };

        lint(cx, expr, sugg, other);
        return true;
    }
    false
}
fn one_extrema<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, recv: &'tcx Expr<'_>, arg: &'tcx Expr<'_>) -> bool {
    if let Some(extrema) = detect_extrema(cx, recv) {
        match extrema {
            Extrema::Minimum => lint(cx, expr, recv.span, arg.span),
            Extrema::Maximum => lint(cx, expr, arg.span, recv.span),
        }
        return true;
    } else if let Some(extrema) = detect_extrema(cx, arg) {
        match extrema {
            Extrema::Minimum => lint(cx, expr, arg.span, recv.span),
            Extrema::Maximum => lint(cx, expr, recv.span, arg.span),
        }
        return true;
    }

    false
}
