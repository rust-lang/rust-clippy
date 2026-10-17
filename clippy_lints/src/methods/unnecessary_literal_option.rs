use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::{MaybeDef as _, MaybeQPath as _};
use clippy_utils::{is_none_expr, sym};
use rustc_hir::{self as hir};
use rustc_lint::LateContext;
use rustc_span::Symbol;

use crate::methods::UNNECESSARY_LITERAL_OPTION;

pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>, method: Symbol) {
    if !recv.span.from_expansion()
        && let init = clippy_utils::expr_or_init(cx, recv)
        && !init.span.from_expansion()
        && let Some(constructor) = literal_constructor(cx, init)
        && let Some(suggestion_message) = note_message(constructor, method)
    {
        let help_message = format!("used `{method}()` on `{constructor}` value");
        span_lint_and_then(cx, UNNECESSARY_LITERAL_OPTION, expr.span, help_message, |diag| {
            diag.span_label(expr.span, suggestion_message);
            diag.help("this is likely a logic error or leftover code, so double check the logic");
        });
    }
}

fn literal_constructor(cx: &LateContext<'_>, init: &hir::Expr<'_>) -> Option<Symbol> {
    if let hir::ExprKind::Call(call, _) = init.kind {
        if let Some((qpath, hir_id)) = call.opt_qpath()
            && let Some(did) = cx.qpath_res(qpath, hir_id).ctor_parent(cx).opt_def_id()
            && Some(did) == cx.tcx.lang_items().option_some_variant()
        {
            Some(sym::Some)
        } else {
            None
        }
    } else if is_none_expr(cx, init) {
        Some(sym::None)
    } else {
        None
    }
}

fn note_message(constructor: Symbol, method: Symbol) -> Option<&'static str> {
    Some(match (constructor, method) {
        (sym::Some, sym::is_some) | (sym::None, sym::is_none) => "this always evaluates to `true`",
        (sym::Some, sym::is_none) | (sym::None, sym::is_some) => "this always evaluates to `false`",
        (sym::None, sym::and | sym::and_then | sym::map) => "this always evaluates to `None`",
        (sym::Some, sym::or | sym::or_else) => "this always evaluates to the original `Some` value",
        _ => return None,
    })
}
