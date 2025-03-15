use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::get_expr_use_or_unification_node;
use clippy_utils::source::{snippet, snippet_with_applicability};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, Node};
use rustc_lint::{LateContext, LintContext};
use rustc_span::sym;

use super::{STRING_FROM_UTF8_AS_BYTES, STRING_LIT_AS_BYTES};
// Max length a b"foo" string can take
const MAX_LENGTH_BYTE_STRING_LIT: usize = 32;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, receiver: &'tcx Expr<'_>) {
    if !expr.span.in_external_macro(cx.sess().source_map())
        && let ExprKind::Lit(lit) = &receiver.kind
        && let LitKind::Str(lit_content, _) = &lit.node
    {
        let callsite = snippet(cx, receiver.span.source_callsite(), r#""foo""#);
        let mut applicability = Applicability::MachineApplicable;
        if callsite.starts_with("include_str!") {
            span_lint_and_sugg(
                cx,
                STRING_LIT_AS_BYTES,
                expr.span,
                "calling `as_bytes()` on `include_str!(..)`",
                "consider using `include_bytes!(..)` instead",
                snippet_with_applicability(cx, receiver.span.source_callsite(), r#""foo""#, &mut applicability)
                    .replacen("include_str", "include_bytes", 1),
                applicability,
            );
        } else if lit_content.as_str().is_ascii()
            && lit_content.as_str().len() <= MAX_LENGTH_BYTE_STRING_LIT
            && !receiver.span.from_expansion()
        {
            if let Some((parent, id)) = get_expr_use_or_unification_node(cx.tcx, expr)
                && let Node::Expr(parent) = parent
                && let ExprKind::Match(scrutinee, ..) = parent.kind
                && scrutinee.hir_id == id
            {
                // Don't lint. Byte strings produce `&[u8; N]` whereas `as_bytes()` produces
                // `&[u8]`. This change would prevent matching with different sized slices.
            } else if !callsite.starts_with("env!") {
                span_lint_and_sugg(
                    cx,
                    STRING_LIT_AS_BYTES,
                    expr.span,
                    "calling `as_bytes()` on a string literal",
                    "consider using a byte string literal instead",
                    format!(
                        "b{}",
                        snippet_with_applicability(cx, receiver.span, r#""foo""#, &mut applicability)
                    ),
                    applicability,
                );
            }
        }
    }
}

pub(super) fn check_into_bytes<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, recv: &'tcx Expr<'_>) {
    if let ExprKind::MethodCall(path, recv, [], _) = &recv.kind
        && matches!(path.ident.name.as_str(), "to_owned" | "to_string")
        && let ExprKind::Lit(lit) = &recv.kind
        && let LitKind::Str(lit_content, _) = &lit.node
        && lit_content.as_str().is_ascii()
        && lit_content.as_str().len() <= MAX_LENGTH_BYTE_STRING_LIT
        && !recv.span.from_expansion()
    {
        let mut applicability = Applicability::MachineApplicable;

        span_lint_and_sugg(
            cx,
            STRING_LIT_AS_BYTES,
            expr.span,
            "calling `into_bytes()` on a string literal",
            "consider using a byte string literal instead",
            format!(
                "b{}.to_vec()",
                snippet_with_applicability(cx, recv.span, r#""..""#, &mut applicability)
            ),
            applicability,
        );
    }
}

// Questa funzione gestisce il caso del `from_utf8` proveniente dalla funzione originale
pub(super) fn check_from_utf8<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    fun: &'tcx Expr<'_>,
    args: &'tcx [Expr<'_>],
) {
    use clippy_utils::{method_calls, path_def_id, sym};
    use rustc_hir::{BorrowKind, LangItem, QPath};

    

    // Find `std::str::converts::from_utf8` or `std::primitive::str::from_utf8`
    if let Some(sym::str_from_utf8 | sym::str_inherent_from_utf8) =
        path_def_id(cx, fun).and_then(|id| cx.tcx.get_diagnostic_name(id))

        // Find string::as_bytes
        && let ExprKind::AddrOf(BorrowKind::Ref, _, args) = args.get(0).unwrap().kind
        && let ExprKind::Index(left, right, _) = args.kind
        && let (method_names, expressions, _) = method_calls(left, 1)
        && method_names == [sym!(as_bytes)]
        && expressions.len() == 1
        && expressions[0].1.is_empty()

        // Check for slicer
        && let ExprKind::Struct(QPath::LangItem(LangItem::Range, ..), _, _) = right.kind
    {
        let mut applicability = Applicability::MachineApplicable;
        let string_expression = &expressions[0].0;

        let snippet_app = snippet_with_applicability(cx, string_expression.span, "..", &mut applicability);

        span_lint_and_sugg(
            cx,
            STRING_FROM_UTF8_AS_BYTES,
            expr.span,
            "calling a slice of `as_bytes()` with `from_utf8` should be not necessary",
            "try",
            format!("Some(&{snippet_app}[{}])", snippet(cx, right.span, "..")),
            applicability,
        );
    }
}
