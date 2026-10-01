use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::{is_expr_temporary_value, sym};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol};

use super::COW_TO_OWNED;

pub fn check(cx: &LateContext<'_>, method_name: Symbol, expr: &Expr<'_>, recv: &Expr<'_>, span: Span) {
    // To avoid accidental move, only temporary values are currently being checked.
    // See https://github.com/rust-lang/rust-clippy/issues/2387#issuecomment-5473343990
    if let Some(method_parent_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id).opt_parent(cx)
        && cx.typeck_results().expr_ty(recv).is_diag_item(cx, sym::Cow)
        && is_to_owned_like(cx, method_name, method_parent_id)
        && is_expr_temporary_value(cx, recv)
    {
        let msg = format!(
            "method `{}` has useless allocation on `Cow::Owned`",
            method_name.as_str()
        );
        let sugg = "into_owned".to_owned();
        let app = Applicability::MachineApplicable;
        span_lint_and_sugg(cx, COW_TO_OWNED, span, msg, "try", sugg, app);
    }
}

fn is_to_owned_like(cx: &LateContext<'_>, method_name: Symbol, method_parent_id: rustc_hir::def_id::DefId) -> bool {
    match method_name {
        sym::to_os_string => method_parent_id.opt_impl_ty(cx).is_diag_item(cx, sym::OsStr),
        sym::to_path_buf => method_parent_id.opt_impl_ty(cx).is_diag_item(cx, sym::Path),
        sym::to_string => method_parent_id.is_diag_item(cx, sym::ToString),
        sym::to_vec => method_parent_id
            .opt_impl_ty(cx)
            .is_some_and(|ty| ty.instantiate_identity().skip_norm_wip().is_slice()),
        _ => false,
    }
}
