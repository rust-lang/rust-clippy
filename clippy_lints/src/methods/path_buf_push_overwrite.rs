use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet;
use clippy_utils::{expr_or_init, sym};
use rustc_ast::StrStyle;
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;

use super::PATH_BUF_PUSH_OVERWRITE;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, push_arg: &'tcx Expr<'tcx>) {
    let ty = cx.typeck_results().expr_ty(recv).peel_refs();
    if matches!(ty.opt_diag_name(cx), Some(sym::PathBuf))
        && let ExprKind::Lit(spanned) = expr_or_init(cx, push_arg).kind
        && let LitKind::Str(symbol, style) = spanned.node
        && let sym_str = symbol.as_str()
        && sym_str.starts_with(['/', '\\'])
    {
        span_lint_and_then(
            cx,
            PATH_BUF_PUSH_OVERWRITE,
            push_arg.span,
            "argument to `PathBuf::push` starts with a path separator",
            |diag| {
                let arg_str = snippet(cx, spanned.span, "..");

                let no_separator = if sym_str.starts_with('/') {
                    arg_str.replacen('/', "", 1)
                } else if let StrStyle::Raw(_) = style {
                    arg_str.replacen('\\', "", 1)
                } else {
                    arg_str.replacen("\\\\", "", 1)
                };

                diag.note("calling push with a starting separator will replace the path instead")
                    .span_suggestion(
                        spanned.span,
                        "if this is unintentional, try removing the starting separator",
                        no_separator,
                        Applicability::Unspecified,
                    );
            },
        );
    }
}
