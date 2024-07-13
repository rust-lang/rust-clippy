use clippy_utils::consts::{constant, Constant};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::sugg::Sugg;
use clippy_utils::visitors::{for_each_expr_without_closures, is_const_evaluatable};
use clippy_utils::{get_item_name, get_named_const_def_id, path_res, peel_hir_expr_while, SpanlessEq};
use core::ops::ControlFlow;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{BinOpKind, BorrowKind, Expr, ExprKind, Safety, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty, TypeFlags, TypeVisitableExt};

use super::{FloatCmpConfig, FLOAT_CMP};

pub(crate) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    config: &FloatCmpConfig,
    expr: &'tcx Expr<'_>,
    op: BinOpKind,
    left: &'tcx Expr<'_>,
    right: &'tcx Expr<'_>,
) {
    let peel_expr = |e: &'tcx Expr<'tcx>| match e.kind {
        ExprKind::Cast(e, _) | ExprKind::AddrOf(BorrowKind::Ref, _, e) | ExprKind::Unary(UnOp::Neg, e) => Some(e),
        _ => None,
    };

    if matches!(op, BinOpKind::Eq | BinOpKind::Ne)
        && let left_reduced = peel_hir_expr_while(left, peel_expr)
        && let right_reduced = peel_hir_expr_while(right, peel_expr)
        && is_float(cx, left_reduced)
        // Don't lint literal comparisons
        && !(matches!(left_reduced.kind, ExprKind::Lit(_)) && matches!(right_reduced.kind, ExprKind::Lit(_)))
        // Allow comparing the results of signum()
        && !(is_signum(cx, left_reduced) && is_signum(cx, right_reduced))
        && match (path_res(cx, left_reduced), path_res(cx, right_reduced)) {
            (Res::Err, _) | (_, Res::Err) => true,
            (left, right) => left != right,
        }
    {
        let left_c = constant(cx, cx.typeck_results(), left_reduced);
        let is_left_const = left_c.is_some();
        if left_c.is_some_and(|c| is_allowed(&c)) {
            return;
        }
        let right_c = constant(cx, cx.typeck_results(), right_reduced);
        let is_right_const = right_c.is_some();
        if right_c.is_some_and(|c| is_allowed(&c)) {
            return;
        }

        if config.ignore_constant_comparisons
            && (is_left_const || is_const_evaluatable(cx, left_reduced))
            && (is_right_const || is_const_evaluatable(cx, right_reduced))
        {
            return;
        }

        if get_named_const_def_id(cx, left_reduced).is_some_and(|id| config.allowed_constants.contains(&id))
            || get_named_const_def_id(cx, right_reduced).is_some_and(|id| config.allowed_constants.contains(&id))
        {
            return;
        }

        if config.ignore_change_detection
            && ((is_pure_expr(cx, left_reduced) && contains_expr(cx, right, left))
                || (is_pure_expr(cx, right_reduced) && contains_expr(cx, left, right)))
        {
            return;
        }

        if let Some(name) = get_item_name(cx, expr) {
            let name = name.as_str();
            if name == "eq" || name == "ne" || name.starts_with("eq_") || name.ends_with("_eq") {
                return;
            }
        }
        let is_comparing_arrays = is_array(cx, left_reduced) || is_array(cx, right_reduced);
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

// This is a best effort guess and may have false positives and negatives.
fn is_pure_expr<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) -> bool {
    match e.kind {
        ExprKind::Path(_) | ExprKind::Lit(_) => true,
        ExprKind::Field(e, _) | ExprKind::Cast(e, _) | ExprKind::Repeat(e, _) => is_pure_expr(cx, e),
        ExprKind::Tup(args) => args.iter().all(|arg| is_pure_expr(cx, arg)),
        ExprKind::Struct(_, fields, base) => {
            base.map_or(true, |base| is_pure_expr(cx, base)) && fields.iter().all(|f| is_pure_expr(cx, f.expr))
        },

        // Since rust doesn't actually have the concept of a pure function we
        // have to guess whether it's likely pure from the signature of the
        // function.
        ExprKind::Unary(_, e) => is_pure_arg_ty(cx, cx.typeck_results().expr_ty_adjusted(e)) && is_pure_expr(cx, e),
        ExprKind::Binary(_, x, y) | ExprKind::Index(x, y, _) => {
            is_pure_arg_ty(cx, cx.typeck_results().expr_ty_adjusted(x))
                && is_pure_arg_ty(cx, cx.typeck_results().expr_ty_adjusted(y))
                && is_pure_expr(cx, x)
                && is_pure_expr(cx, y)
        },
        ExprKind::MethodCall(_, recv, args, _) => {
            is_pure_arg_ty(cx, cx.typeck_results().expr_ty_adjusted(recv))
                && is_pure_expr(cx, recv)
                && cx
                    .typeck_results()
                    .type_dependent_def_id(e.hir_id)
                    .is_some_and(|did| matches!(cx.tcx.fn_sig(did).skip_binder().skip_binder().safety, Safety::Safe))
                && args
                    .iter()
                    .all(|arg| is_pure_arg_ty(cx, cx.typeck_results().expr_ty_adjusted(arg)) && is_pure_expr(cx, arg))
        },
        ExprKind::Call(f, args @ [_, ..]) => {
            is_pure_expr(cx, f)
                && is_pure_fn_ty(cx, cx.typeck_results().expr_ty_adjusted(f))
                && args
                    .iter()
                    .all(|arg| is_pure_arg_ty(cx, cx.typeck_results().expr_ty_adjusted(arg)) && is_pure_expr(cx, arg))
        },

        _ => false,
    }
}

fn is_pure_fn_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    let sig = match *ty.peel_refs().kind() {
        ty::FnDef(did, _) => cx.tcx.fn_sig(did).skip_binder(),
        ty::FnPtr(sig) => sig,
        ty::Closure(_, args) => {
            return args.as_closure().upvar_tys().iter().all(|ty| is_pure_arg_ty(cx, ty));
        },
        _ => return false,
    };
    matches!(sig.skip_binder().safety, Safety::Safe)
}

fn is_pure_arg_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    !ty.is_mutable_ptr()
        && ty.is_copy_modulo_regions(cx.tcx, cx.param_env)
        && (ty.peel_refs().is_freeze(cx.tcx, cx.param_env)
            || !ty.has_type_flags(TypeFlags::HAS_FREE_REGIONS | TypeFlags::HAS_RE_ERASED | TypeFlags::HAS_RE_BOUND))
}

fn contains_expr<'tcx>(cx: &LateContext<'tcx>, corpus: &'tcx Expr<'tcx>, e: &'tcx Expr<'tcx>) -> bool {
    for_each_expr_without_closures(corpus, |corpus| {
        if SpanlessEq::new(cx).eq_expr(corpus, e) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_some()
}

// Return true if `expr` is the result of `signum()` invoked on a float value.
fn is_signum(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let ExprKind::MethodCall(method_name, self_arg, ..) = expr.kind
        && sym!(signum) == method_name.ident.name
    {
        // Check that the receiver of the signum() is a float
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
