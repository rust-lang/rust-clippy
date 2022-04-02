//! Lint for `some_result_or_option.unwrap_or_else(Default::default)`

use super::UNWRAP_OR_ELSE_DEFAULT;
use clippy_macros::expr_sugg;
use clippy_utils::{_internal::lint_expr_and_sugg, is_default_equivalent_call, ty::is_type_diagnostic_item};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'_>,
    recv: &'tcx hir::Expr<'_>,
    u_arg: &'tcx hir::Expr<'_>,
) {
    // something.unwrap_or_else(Default::default)
    // ^^^^^^^^^- recv          ^^^^^^^^^^^^^^^^- u_arg
    // ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^- expr
    let recv_ty = cx.typeck_results().expr_ty(recv);
    let is_option = is_type_diagnostic_item(cx, recv_ty, sym::Option);
    let is_result = is_type_diagnostic_item(cx, recv_ty, sym::Result);

    if_chain! {
        if is_option || is_result;
        if is_default_equivalent_call(cx, u_arg);
        then {
            lint_expr_and_sugg(
                cx,
                UNWRAP_OR_ELSE_DEFAULT,
                "use of `.unwrap_or_else(..)` to construct default value",
                expr,
                "try",
                expr_sugg!({}.unwrap_or_default(), recv),
                Applicability::MachineApplicable,
            );
        }
    }
}
