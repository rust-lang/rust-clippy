use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_context;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;

use super::STABLE_SORT;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, e: &'tcx Expr<'_>, recv: &'tcx Expr<'_>) {
    span_lint_and_then(
        cx,
        STABLE_SORT,
        e.span,
        &format!("used `sort` over `sort_unstable`"),
        |diag| {
            let mut app = Applicability::MachineApplicable;
            let recv_snip = snippet_with_context(cx, recv.span, e.span.ctxt(), "..", &mut app).0;
            diag.span_suggestion(e.span, "try", format!("{recv_snip}.sort_unstable()"), app);
            diag.note(
                "an unstable sort typically performs faster without any observable difference for this data type",
            );
        },
    );
}
