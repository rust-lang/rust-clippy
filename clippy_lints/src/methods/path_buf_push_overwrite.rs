use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeDef as _;
use clippy_utils::source::snippet;
use clippy_utils::sugg::Sugg;
use clippy_utils::{expr_or_init, sym};
use rustc_ast::ast::{LitKind, StrStyle};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty::adjustment::Adjust;
use rustc_span::Span;

use super::PATH_BUF_PUSH_OVERWRITE;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, recv: &'tcx Expr<'tcx>, push_arg: &'tcx Expr<'tcx>, expr_span: Span) {
    // Shares logic with `join_absolute_paths`
    let ty = cx.typeck_results().expr_ty_adjusted(recv).peel_refs();
    if ty.is_diag_item(cx, sym::PathBuf)
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

                diag.note("pushing a path starting with separator will replace the path instead")
                    .span_suggestion(
                        spanned.span,
                        "if this is unintentional, try removing the starting separator",
                        no_separator,
                        Applicability::Unspecified,
                    );

                let derefs = cx
                    .typeck_results()
                    .expr_adjustments(recv)
                    .iter()
                    .filter(|adj| matches!(adj.kind, Adjust::Deref(_)))
                    .count();
                // Assignable targets are a syntactic place like `path` or `self.path`, or a dereferenced
                // temporary like `*cell.borrow_mut()`, but not `PathBuf::new()`
                if recv.is_syntactic_place_expr() || derefs > 0 {
                    let mut app = Applicability::Unspecified;
                    let target = (0..derefs).fold(
                        Sugg::hir_with_context(cx, recv, expr_span.ctxt(), "..", &mut app),
                        |sugg, _| sugg.deref(),
                    );
                    diag.span_suggestion(
                        expr_span,
                        "if this is intentional, consider using `PathBuf::from`",
                        format!("{target} = PathBuf::from({arg_str})"),
                        app,
                    );
                }
            },
        );
    }
}
