use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{eager_or_lazy, usage};
use rustc_errors::emitter::MAX_SUGGESTION_HIGHLIGHT_LINES;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

use super::UNNECESSARY_LAZY_EVALUATIONS;

/// lint use of `<fn>_else(simple closure)` for `Option`s and `Result`s that can be
/// replaced with `<fn>(return value of simple closure)`
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx hir::Expr<'_>,
    recv: &'tcx hir::Expr<'_>,
    arg: &'tcx hir::Expr<'_>,
    simplify_using: &str,
) {
    let is_option = is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv), sym::Option);
    let is_result = is_type_diagnostic_item(cx, cx.typeck_results().expr_ty(recv), sym::Result);

    if is_option || is_result {
        if let hir::ExprKind::Closure(_, _, eid, _, _) = arg.kind {
            let body = cx.tcx.hir().body(eid);
            let body_expr = &body.value;

            if usage::BindingUsageFinder::are_params_used(cx, body) {
                return;
            }

            if eager_or_lazy::switch_to_eager_eval(cx, body_expr) {
                let msg = if is_option {
                    "unnecessary closure used to substitute value for `Option::None`"
                } else {
                    "unnecessary closure used to substitute value for `Result::Err`"
                };
                let applicability = if body
                    .params
                    .iter()
                    // bindings are checked to be unused above
                    .all(|param| matches!(param.pat.kind, hir::PatKind::Binding(..) | hir::PatKind::Wild))
                {
                    Applicability::MachineApplicable
                } else {
                    // replacing the lambda may break type inference
                    Applicability::MaybeIncorrect
                };

                let mut sugg_span = expr.span;
                let mut sugg: String = format!(
                    "{}.{}({})",
                    snippet(cx, recv.span, ".."),
                    simplify_using,
                    snippet(cx, body_expr.span, ".."),
                );
                let mut help = format!("use `{}` instead", simplify_using);

                if sugg.lines().count() > MAX_SUGGESTION_HIGHLIGHT_LINES {
                    sugg_span = expr.span.with_lo(recv.span.hi());
                    sugg = format!(".{}({})", simplify_using, snippet(cx, body_expr.span, ".."));
                    help = "try this".to_string();
                }

                span_lint_and_sugg(
                    cx,
                    UNNECESSARY_LAZY_EVALUATIONS,
                    sugg_span,
                    msg,
                    &help,
                    sugg,
                    applicability,
                );
            }
        }
    }
}
