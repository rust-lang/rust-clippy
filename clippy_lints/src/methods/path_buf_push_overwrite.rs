use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::{expr_or_init, sym};
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;

use super::PATH_BUF_PUSH_OVERWRITE;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, push_arg: &'tcx Expr<'tcx>) {
    let ty = cx.typeck_results().expr_ty(recv).peel_refs();
    if matches!(ty.opt_diag_name(cx), Some(sym::PathBuf))
        && let ExprKind::Lit(spanned) = expr_or_init(cx, push_arg).kind
        && let LitKind::Str(symbol, _style) = spanned.node
        && let sym_str = symbol.as_str()
        && sym_str.starts_with(['/', '\\'])
    {
        span_lint_and_sugg(
            cx,
            PATH_BUF_PUSH_OVERWRITE,
            spanned.span,
            "calling `push` with '/' or '\\' (file system root) will overwrite the previous path definition",
            "try",
            format!("\"{}\"", sym_str.trim_start_matches(['/', '\\'])),
            Applicability::MaybeIncorrect,
        );
    }
}
