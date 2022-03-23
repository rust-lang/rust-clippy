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
    span: Span,
) {
    let applicability = Applicability::MachineApplicable;
    let collect_output_adjusted_type = &context.typeck_results().expr_ty_adjusted(join_self_arg);
    if_chain! {
        // the turbofish for collect is ::<Vec<String>>
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
                span.with_hi(expr.span.hi()),
                r#"called `.collect<Vec<String>>().join("")` on an iterator"#,
                "try using",
                "collect::<String>()".to_owned(),
                applicability,
            );
        }
    }
}
