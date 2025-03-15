use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::is_type_lang_item;
use rustc_errors::Applicability;
use rustc_hir::{Expr, LangItem};
use rustc_lint::LateContext;
use rustc_middle::ty; // Importa il modulo ty

use super::{STR_TO_STRING, STRING_TO_STRING};

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    method_name: &str,
    receiver: &'tcx Expr<'_>,
    _args: &[Expr<'_>],
) {
    if method_name != "to_string" {
        return;
    }

    let ty = cx.typeck_results().expr_ty(receiver);

    // &str -> String
    if let ty::Ref(_, ty, ..) = ty.kind() { // Usa ty::Ref invece di rustc_middle::ty::TyKind::Ref
        if ty.is_str() {
            span_lint_and_then(
                cx,
                STR_TO_STRING,
                expr.span,
                "`to_string()` called on a `&str`",
                |diag| {
                    let mut applicability = Applicability::MachineApplicable;
                    let snippet = snippet_with_applicability(cx, receiver.span, "..", &mut applicability);
                    diag.span_suggestion(expr.span, "try", format!("{snippet}.to_owned()"), applicability);
                },
            );
        }
    }

    // String -> String
    if is_type_lang_item(cx, ty, LangItem::String) {
        #[expect(clippy::collapsible_span_lint_calls, reason = "rust-clippy#7797")]
        span_lint_and_then(
            cx,
            STRING_TO_STRING,
            expr.span,
            "`to_string()` called on a `String`",
            |diag| {
                diag.help("consider using `.clone()`");
            },
        );
    }
}
