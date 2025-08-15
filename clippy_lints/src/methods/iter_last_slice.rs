use super::ITER_LAST_SLICE;
use super::utils::derefs_to_slice;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher;
use clippy_utils::source::snippet_with_applicability;
use rustc_ast::ast;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'_>, caller_expr: &'tcx hir::Expr<'_>) {
    if derefs_to_slice(cx, caller_expr, cx.typeck_results().expr_ty(caller_expr)).is_some() {
        // caller is a Slice
        if let hir::ExprKind::Index(caller_var, index_expr, _) = &caller_expr.kind
            && let Some(higher::Range {
                start: _,
                end: Some(end_expr),
                limits,
            }) = higher::Range::hir(index_expr)
            && let hir::ExprKind::Lit(end_lit) = &end_expr.kind
            && let ast::LitKind::Int(end_idx, _) = end_lit.node
        {
            let mut applicability = Applicability::MachineApplicable;
            let suggest = if end_idx == 0 {
                // This corresponds to `$slice[..0].iter().last()`, which
                // evaluates to `None`. Maybe this is worth its own lint?
                format!(
                    "{}.last()",
                    snippet_with_applicability(cx, caller_var.span, "..", &mut applicability)
                )
            } else {
                let idx = match limits {
                    ast::RangeLimits::HalfOpen => end_idx.get() - 1,
                    ast::RangeLimits::Closed => end_idx.get(),
                };

                format!(
                    "{}.get({idx})",
                    snippet_with_applicability(cx, caller_var.span, "..", &mut applicability)
                )
            };
            span_lint_and_sugg(
                cx,
                ITER_LAST_SLICE,
                expr.span,
                "using `.iter().last()` on a slice without end index",
                "try calling",
                suggest,
                applicability,
            );
        }
    } else if super::iter_next_slice::is_vec_or_array(cx, caller_expr) {
        // caller is a Vec or an Array
        let mut applicability = Applicability::MachineApplicable;
        span_lint_and_sugg(
            cx,
            ITER_LAST_SLICE,
            expr.span,
            "using `.iter().last()` on an array",
            "try calling",
            format!(
                "{}.last()",
                snippet_with_applicability(cx, caller_expr.span, "..", &mut applicability)
            ),
            applicability,
        );
    }
}
