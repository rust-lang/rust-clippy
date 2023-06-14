use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_trait_method;
use clippy_utils::source::snippet_with_applicability;
use rustc_ast::Mutability;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Node};
use rustc_lint::LateContext;
use rustc_span::sym;
use rustc_span::Span;
use std::borrow::Cow;

use super::ITER_SKIP;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, arg: &Expr<'_>, iter_method: &str, trim_span: Span) {
    let mutability = match iter_method {
        "iter" | "into_iter" => Mutability::Not,
        "iter_mut" => Mutability::Mut,
        _ => unreachable!("pattern in `mod.rs` only matches for these"),
    };

    if is_trait_method(cx, expr, sym::Iterator) {
        let needs_iter = if let Node::Expr(next_call) = cx.tcx.hir().get_parent(expr.hir_id)
            && let ExprKind::MethodCall(..) = next_call.kind
        {
            true
        } else {
            false
        };

        let mut app = Applicability::MachineApplicable;
        span_lint_and_then(
            cx,
            ITER_SKIP,
            expr.span.with_lo(trim_span.hi()),
            &format!("called `{iter_method}()` followed by `skip(..)`"),
            |diag| {
                diag.span_suggestion(
                    expr.span.with_lo(trim_span.hi()),
                    "use `get` instead",
                    format!(
                        ".get{}({}..).unwrap_or(&{}[]){}",
                        if mutability.is_mut() { "_mut" } else { "" },
                        snippet_with_applicability(cx, arg.span, "<i>", &mut app),
                        if mutability.is_mut() { "mut " } else { "" },
                        // If there's another call after `skip`, we must convert it to an iterator
                        if needs_iter {
                            format!(".{iter_method}()").into()
                        } else {
                            Cow::Borrowed("")
                        },
                    ),
                    app,
                );
                diag.note("you may need to call `copied`");
            },
        );
    }
}
