use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::pat_is_wild;
use clippy_utils::res::{MaybeDef, MaybeResPath};
use clippy_utils::source::{indent_of, reindent_multiline, snippet_with_applicability, snippet_with_context};
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
    msrv: Msrv,
) {
    if !msrv.meets(cx, msrvs::BOOL_THEN_SOME) {
        return;
    }
    let mut applicability = Applicability::MaybeIncorrect;
    let value = match recv.kind {
        ExprKind::Call(f, [value]) if f.basic_res().ctor_parent(cx).is_lang_item(cx, OptionSome) => value,
        _ => return,
    };
    let condition = if let ExprKind::Closure(c) = arg.kind
        && let Body {
            params: [p],
            value: condition,
        } = cx.tcx.hir_body(c.body)
        && pat_is_wild(cx, &p.pat.kind, arg)
    {
        condition
    } else {
        return;
    };
    let (condition_text, condition_is_macro) =
        snippet_with_context(cx, condition.span, arg.span.ctxt(), "..", &mut applicability);
    let parentheses = !condition_is_macro && cx.precedence(condition) < ExprPrecedence::Unambiguous;
    let value_text = snippet_with_applicability(cx, value.span, "..", &mut applicability);
    let sugg = format!(
        "{}{condition_text}{}.then_some({value_text})",
        if parentheses { "(" } else { "" },
        if parentheses { ")" } else { "" },
    );
    span_lint_and_then(cx, FILTER_SOME, expr.span, "use of `Some(_).filter(_)`", |diag| {
        diag.span_suggestion(
            expr.span,
            "consider using `then_some` instead",
            reindent_multiline(&sugg, true, indent_of(cx, expr.span)),
            applicability,
        );
        diag.note(
            "this change will alter the order in which the condition and the \
             value are evaluated",
        );
    });
}
