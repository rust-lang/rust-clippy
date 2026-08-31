use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::{is_expr_temporary_value, sym};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol};

use crate::methods::is_clone_like;

use super::COW_TO_OWNED;

pub fn check(cx: &LateContext<'_>, method_name: Symbol, expr: &Expr<'_>, recv: &Expr<'_>, span: Span) {
    if let Some(method_parent_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id).opt_parent(cx)
        && cx.typeck_results().expr_ty(recv).is_diag_item(cx, sym::Cow)
        && is_clone_like(cx, method_name, method_parent_id)
        && method_name != sym::to_owned // hit Cow::to_owned
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
