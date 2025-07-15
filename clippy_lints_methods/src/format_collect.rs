use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::macros::{is_format_macro, root_macro_call_first_node};
use clippy_utils::ty::is_type_lang_item;
use rustc_hir::{Expr, ExprKind, LangItem};
use rustc_lint::LateContext;
use rustc_span::Span;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for usage of `.map(|_| format!(..)).collect::<String>()`.
    ///
    /// ### Why is this bad?
    /// This allocates a new string for every element in the iterator.
    /// This can be done more efficiently by creating the `String` once and appending to it in `Iterator::fold`,
    /// using either the `write!` macro which supports exactly the same syntax as the `format!` macro,
    /// or concatenating with `+` in case the iterator yields `&str`/`String`.
    ///
    /// Note also that `write!`-ing into a `String` can never fail, despite the return type of `write!` being `std::fmt::Result`,
    /// so it can be safely ignored or unwrapped.
    ///
    /// ### Example
    /// ```no_run
    /// fn hex_encode(bytes: &[u8]) -> String {
    ///     bytes.iter().map(|b| format!("{b:02X}")).collect()
    /// }
    /// ```
    /// Use instead:
    /// ```no_run
    /// use std::fmt::Write;
    /// fn hex_encode(bytes: &[u8]) -> String {
    ///     bytes.iter().fold(String::new(), |mut output, b| {
    ///         let _ = write!(output, "{b:02X}");
    ///         output
    ///     })
    /// }
    /// ```
    #[clippy::version = "1.73.0"]
    pub FORMAT_COLLECT,
    pedantic,
    "`format!`ing every element in a collection, then collecting the strings into a new `String`"
}

/// Same as `peel_blocks` but only actually considers blocks that are not from an expansion.
/// This is needed because always calling `peel_blocks` would otherwise remove parts of the
/// `format!` macro, which would cause `root_macro_call_first_node` to return `None`.
fn peel_non_expn_blocks<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match expr.kind {
        ExprKind::Block(block, _) if !expr.span.from_expansion() => peel_non_expn_blocks(block.expr?),
        _ => Some(expr),
    }
}

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, map_arg: &Expr<'_>, map_span: Span) {
    if is_type_lang_item(cx, cx.typeck_results().expr_ty(expr), LangItem::String)
        && let ExprKind::Closure(closure) = map_arg.kind
        && let body = cx.tcx.hir_body(closure.body)
        && let Some(value) = peel_non_expn_blocks(body.value)
        && let Some(mac) = root_macro_call_first_node(cx, value)
        && is_format_macro(cx, mac.def_id)
    {
        span_lint_and_then(
            cx,
            FORMAT_COLLECT,
            expr.span,
            "use of `format!` to build up a string from an iterator",
            |diag| {
                diag.span_help(map_span, "call `fold` instead")
                    .span_help(value.span.source_callsite(), "... and use the `write!` macro here")
                    .note("this can be written more efficiently by appending to a `String` directly");
            },
        );
    }
}
