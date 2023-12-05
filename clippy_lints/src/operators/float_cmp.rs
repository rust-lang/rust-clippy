use clippy_utils::consts::{constant, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sugg::Sugg;
use clippy_utils::visitors::is_const_evaluatable;
use clippy_utils::{get_item_name, is_expr_named_const, peel_hir_expr_while};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, BorrowKind, Expr, ExprKind, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty;

use super::{FloatCmpConfig, FLOAT_CMP};

pub(crate) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    config: FloatCmpConfig,
    expr: &'tcx Expr<'_>,
    op: BinOpKind,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
) {
    if (op == BinOpKind::Eq || op == BinOpKind::Ne)
        && is_float(cx, left)
        // Allow comparing the results of signum()
        && !(is_signum(cx, left) && is_signum(cx, right))
    {
        let left_c = constant(cx, cx.typeck_results(), left);
        let is_left_const = left_c.is_some();
        if left_c.is_some_and(|c| is_allowed(&c)) {
            return;
        }
        let right_c = constant(cx, cx.typeck_results(), right);
        let is_right_const = right_c.is_some();
        if right_c.is_some_and(|c| is_allowed(&c)) {
            return;
        }

        if config.ignore_constant_comparisons
            && (is_left_const || is_const_evaluatable(cx, left))
            && (is_right_const || is_const_evaluatable(cx, right))
        {
            return;
        }

        let peel_expr = |e: &'tcx Expr<'tcx>| match e.kind {
            ExprKind::Cast(e, _) | ExprKind::AddrOf(BorrowKind::Ref, _, e) => Some(e),
            _ => None,
        };
        if config.ignore_named_constants
            && (is_expr_named_const(cx, peel_hir_expr_while(left, peel_expr))
                || is_expr_named_const(cx, peel_hir_expr_while(right, peel_expr)))
        {
            return;
        }

        if let Some(name) = get_item_name(cx, expr) {
            let name = name.as_str();
            if name == "eq" || name == "ne" || name == "is_nan" || name.starts_with("eq_") || name.ends_with("_eq") {
                return;
            }
        }
        let is_comparing_arrays = is_array(cx, left) || is_array(cx, right);
        let msg = if is_comparing_arrays {
            "strict comparison of `f32` or `f64` arrays"
        } else {
            "strict comparison of `f32` or `f64`"
        };
        span_lint_and_then(cx, FLOAT_CMP, expr.span, msg, |diag| {
            let lhs = Sugg::hir(cx, left, "..");
            let rhs = Sugg::hir(cx, right, "..");

            if !is_comparing_arrays {
                diag.span_suggestion(
                    expr.span,
                    "consider comparing them within some margin of error",
                    format!(
                        "({}).abs() {} error_margin",
                        lhs - rhs,
                        if op == BinOpKind::Eq { '<' } else { '>' }
                    ),
                    Applicability::HasPlaceholders, // snippet
                );
            }
        });
    }
}

fn is_allowed(val: &Constant<'_>) -> bool {
    match val {
        // FIXME(f16_f128): add when equality check is available on all platforms
        Constant::Ref(val) => is_allowed(val),
        &Constant::F32(f) => f == 0.0 || f.is_infinite(),
        &Constant::F64(f) => f == 0.0 || f.is_infinite(),
        Constant::Vec(vec) => vec.iter().all(|f| match *f {
            Constant::F32(f) => f == 0.0 || f.is_infinite(),
            Constant::F64(f) => f == 0.0 || f.is_infinite(),
            _ => false,
        }),
        Constant::Repeat(val, _) => match **val {
            Constant::F32(f) => f == 0.0 || f.is_infinite(),
            Constant::F64(f) => f == 0.0 || f.is_infinite(),
            _ => false,
        },
        _ => false,
    }
}

// Return true if `expr` is the result of `signum()` invoked on a float value.
fn is_signum(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // The negation of a signum is still a signum
    if let ExprKind::Unary(UnOp::Neg, child_expr) = expr.kind {
        return is_signum(cx, child_expr);
    }

    if let ExprKind::MethodCall(method_name, self_arg, ..) = expr.kind
        && sym!(signum) == method_name.ident.name
    // Check that the receiver of the signum() is a float (expressions[0] is the receiver of
    // the method call)
    {
        return is_float(cx, self_arg);
    }
    false
}

fn is_float(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let value = &cx.typeck_results().expr_ty(expr).peel_refs().kind();

    if let ty::Array(arr_ty, _) = value {
        return matches!(arr_ty.kind(), ty::Float(_));
    };

    matches!(value, ty::Float(_))
}

fn is_array(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(&cx.typeck_results().expr_ty(expr).peel_refs().kind(), ty::Array(_, _))
}
