use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::ty::is_type_diagnostic_item;
use hir::ExprKind;
use rustc_ast::ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{sym, Span};

use super::UNNECESSARY_JOIN;

pub(super) fn check<'tcx>(
    context: &LateContext<'tcx>,
    join_self_arg: &'tcx hir::Expr<'tcx>,
    join_arg: &'tcx hir::Expr<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
    recv2: &'tcx hir::Expr<'tcx>,
) {
    let applicability = Applicability::MachineApplicable;

    if_chain! {
        // the current join method is being called on a vector
        // e.g .join("")
        let collect_output_type = context.typeck_results().expr_ty(join_self_arg);
        // the turbofish for collect is ::<Vec<String>>
        let collect_output_adjusted_type = &context.typeck_results().expr_ty_adjusted(join_self_arg);
        if let ty::Ref(_, ref_type, _) = collect_output_adjusted_type.kind();
        if let ty::Slice(slice) = ref_type.kind();
        if is_type_diagnostic_item(context, *slice, sym::String);
        // the argument for join is ""
        if let ExprKind::Lit(spanned) = &join_arg.kind;
        if let LitKind::Str(symbol, _) = spanned.node;
        if symbol.is_empty();

        then {
            span_lint_and_sugg(
                context,
                UNNECESSARY_JOIN,
                recv2.span.with_lo(expr.span.hi()),
                &format!(
                    "called `.collect<Vec<String>>().join(\"\")` on a {}", collect_output_type,
                ),
                "try using",
                ".collect::<String>()".to_owned(),
                applicability,
            );
        }
    }
}
