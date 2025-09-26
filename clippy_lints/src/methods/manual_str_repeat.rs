use clippy_utils::diagnostics::{applicability_for_ctxt, span_lint_and_sugg};
use clippy_utils::is_path_diagnostic_item;
use clippy_utils::source::{SpanExt, snippet_with_applicability};
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::{is_type_diagnostic_item, is_type_lang_item};
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::symbol::sym;
use std::borrow::Cow;

use super::MANUAL_STR_REPEAT;

enum RepeatKind {
    String,
    Char(char),
}

fn get_ty_param(ty: Ty<'_>) -> Option<Ty<'_>> {
    if let ty::Adt(_, subs) = ty.kind() {
        subs.types().next()
    } else {
        None
    }
}

fn parse_repeat_arg(cx: &LateContext<'_>, e: &Expr<'_>) -> Option<RepeatKind> {
    if let ExprKind::Lit(lit) = &e.kind {
        match lit.node {
            LitKind::Str(..) => Some(RepeatKind::String),
            LitKind::Char(c) => Some(RepeatKind::Char(c)),
            _ => None,
        }
    } else {
        let ty = cx.typeck_results().expr_ty(e);
        if is_type_lang_item(cx, ty, LangItem::String)
            || (is_type_lang_item(cx, ty, LangItem::OwnedBox) && get_ty_param(ty).is_some_and(Ty::is_str))
            || (is_type_diagnostic_item(cx, ty, sym::Cow) && get_ty_param(ty).is_some_and(Ty::is_str))
        {
            Some(RepeatKind::String)
        } else {
            let ty = ty.peel_refs();
            (ty.is_str() || is_type_lang_item(cx, ty, LangItem::String)).then_some(RepeatKind::String)
        }
    }
}

pub(super) fn check(
    cx: &LateContext<'_>,
    collect_expr: &Expr<'_>,
    take_expr: &Expr<'_>,
    take_self_arg: &Expr<'_>,
    take_arg: &Expr<'_>,
) {
    if let ExprKind::Call(repeat_fn, [repeat_arg]) = take_self_arg.kind
        && is_path_diagnostic_item(cx, repeat_fn, sym::iter_repeat)
        && is_type_lang_item(cx, cx.typeck_results().expr_ty(collect_expr), LangItem::String)
        && let Some(take_id) = cx.typeck_results().type_dependent_def_id(take_expr.hir_id)
        && let Some(iter_trait_id) = cx.tcx.get_diagnostic_item(sym::Iterator)
        && cx.tcx.trait_of_assoc(take_id) == Some(iter_trait_id)
        && let Some(repeat_kind) = parse_repeat_arg(cx, repeat_arg)
        && let ctxt = collect_expr.span.ctxt()
        && ctxt == take_expr.span.ctxt()
        && ctxt == take_self_arg.span.ctxt()
        && let Some(count_snip) = take_arg.span.get_source_text_at_ctxt(cx, ctxt)
    {
        let mut app = applicability_for_ctxt(ctxt);
        let val_str = match repeat_kind {
            RepeatKind::Char(_) if repeat_arg.span.ctxt() != ctxt => return,
            RepeatKind::Char('\'') => r#""'""#.into(),
            RepeatKind::Char('"') => r#""\"""#.into(),
            RepeatKind::Char(_) => match snippet_with_applicability(cx, repeat_arg.span, "..", &mut app) {
                Cow::Owned(s) => Cow::Owned(format!("\"{}\"", &s[1..s.len() - 1])),
                s @ Cow::Borrowed(_) => s,
            },
            RepeatKind::String => Sugg::hir_with_context(cx, repeat_arg, ctxt, "..", &mut app)
                .maybe_paren()
                .to_string()
                .into(),
        };

        span_lint_and_sugg(
            cx,
            MANUAL_STR_REPEAT,
            collect_expr.span,
            "manual implementation of `str::repeat` using iterators",
            "try",
            format!("{val_str}.repeat({count_snip})"),
            app,
        );
    }
}
