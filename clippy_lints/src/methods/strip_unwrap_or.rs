use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_span::{Symbol, SyntaxContext};

use super::STRIP_UNWRAP_OR;

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    strip_recv: &Expr<'_>,
    strip_arg: &Expr<'_>,
    strip_method: Symbol,
    unwrap_arg: &Expr<'_>,
    msrv: Msrv,
) {
    if !msrv.meets(cx, msrvs::STR_TRIM_PREFIX)
        || !cx.typeck_results().expr_ty_adjusted(strip_recv).peel_refs().is_str()
        || !eq_expr_value(cx, SyntaxContext::root(), strip_recv, unwrap_arg)
    {
        return;
    }

    let (kind, trim) = if strip_method == sym::strip_prefix {
        ("prefix", sym::trim_prefix)
    } else {
        ("suffix", sym::trim_suffix)
    };

    let mut applicability = Applicability::MachineApplicable;
    let recv_snip = snippet_with_applicability(cx, strip_recv.span, "_", &mut applicability);
    let arg_snip = snippet_with_applicability(cx, strip_arg.span, "_", &mut applicability);
    span_lint_and_sugg(
        cx,
        STRIP_UNWRAP_OR,
        expr.span,
        format!("`strip_{kind}().unwrap_or()` can be written with `trim_{kind}()`"),
        "try",
        format!("{recv_snip}.{trim}({arg_snip})"),
        applicability,
    );
}
