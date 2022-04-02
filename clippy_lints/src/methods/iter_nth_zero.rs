use clippy_macros::expr_sugg;
use clippy_utils::_internal::lint_expr_and_sugg;
use clippy_utils::consts::{constant, Constant};
use clippy_utils::is_trait_method;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

use super::ITER_NTH_ZERO;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'_>,
    recv: &'tcx hir::Expr<'_>,
    arg: &'tcx hir::Expr<'_>,
) {
    if_chain! {
        if is_trait_method(cx, expr, sym::Iterator);
        if let Some((Constant::Int(0), _)) = constant(cx, cx.typeck_results(), arg);
        then {
            lint_expr_and_sugg(
                cx,
                ITER_NTH_ZERO,
                "called `.nth(0)` on a `std::iter::Iterator`, when `.next()` is equivalent",
                expr,
                "try calling `.next()` instead of `.nth(0)`",
                expr_sugg!({}.next(), recv),
                Applicability::MachineApplicable,
            );
        }
    }
}
