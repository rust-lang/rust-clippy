use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::{indent_of, reindent_multiline, snippet};
use clippy_utils::{is_res_lang_ctor, pat_is_wild, path_res};
use rustc_ast::util::parser::ExprPrecedence;
use rustc_errors::Applicability;
use rustc_hir::LangItem::OptionSome;
use rustc_hir::{Body, Expr, ExprKind};
use rustc_lint::LateContext;

use super::FILTER_SOME;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    recv: &'tcx Expr<'tcx>,
    arg: &'tcx Expr<'tcx>,
) {
    let value = match recv.kind {
        ExprKind::Call(f, [value]) if is_res_lang_ctor(cx, path_res(cx, f), OptionSome) => value,
        _ => return,
    };
    let condition = if let ExprKind::Closure(c) = arg.kind
        && let Body {
            params: [p],
            value: condition,
        } = cx.tcx.hir_body(c.body)
        && pat_is_wild(cx, &p.pat.kind, arg)
    {
        Some(condition)
    } else {
        None
    };
    span_lint_and_then(cx, FILTER_SOME, expr.span, "`filter` for `Some`", |diag| {
        let help = "consider using `then_some` instead";
        if let Some(condition) = condition {
            let parentheses = cx.precedence(condition) < ExprPrecedence::Unambiguous;
            diag.span_suggestion(
                expr.span,
                help,
                reindent_multiline(
                    &format!(
                        "{}{}{}.then_some({})",
                        if parentheses { "(" } else { "" },
                        snippet(cx, condition.span, ".."),
                        if parentheses { ")" } else { "" },
                        snippet(cx, value.span, "..")
                    ),
                    true,
                    indent_of(cx, expr.span),
                ),
                Applicability::MaybeIncorrect,
            );
            diag.note(
                "this change will alter the order in which the condition and \
                 the value are evaluated",
            );
        } else {
            diag.help(help);
        }
    });
}
