use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::{expr_or_init, paths};
use rustc_errors::Applicability;
use rustc_lint::LateContext;

use crate::methods::NON_CANONICAL_TOTAL_FLOAT_CMP;

pub(super) fn check(
    cx: &LateContext<'_>,
    full_expr: &rustc_hir::Expr<'_>,
    partial_cmp_recv: &rustc_hir::Expr<'_>,
    partial_cmp_arg: &rustc_hir::Expr<'_>,
    unwrap_or_arg: &rustc_hir::Expr<'_>,
) {
    // format: <float>.partial_cmp(<float>).unwrap_or(<Ordering>)

    if !cx
        .typeck_results()
        .expr_ty(expr_or_init(cx, partial_cmp_recv))
        .peel_refs()
        .is_floating_point()
        || !cx
            .typeck_results()
            .expr_ty(expr_or_init(cx, partial_cmp_arg))
            .peel_refs()
            .is_floating_point()
    {
        return;
    }

    let ordering_variants = [
        &paths::CMP_ORDERING_EQUAL,
        &paths::CMP_ORDERING_GREATER,
        &paths::CMP_ORDERING_LESS,
    ];

    let is_cmp_ordering_variant = ordering_variants
        .iter()
        .any(|variant| variant.matches_path(cx, expr_or_init(cx, unwrap_or_arg)));

    if !is_cmp_ordering_variant {
        return;
    }

    let mut applicability = Applicability::MaybeIncorrect;

    let partial_cmp_recv =
        clippy_utils::source::snippet_with_applicability(cx.tcx.sess, partial_cmp_recv.span, "..", &mut applicability);

    let partial_cmp_arg =
        clippy_utils::source::snippet_with_applicability(cx.tcx.sess, partial_cmp_arg.span, "..", &mut applicability);

    span_lint_and_sugg(
        cx,
        NON_CANONICAL_TOTAL_FLOAT_CMP,
        full_expr.span,
        "non-canonical total float comparison",
        "try",
        format!("{partial_cmp_recv}.total_cmp({partial_cmp_arg})"),
        applicability,
    );
}
