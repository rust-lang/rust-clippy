use clippy_utils::ty::is_type_diagnostic_item;
use clippy_utils::{get_parent_expr, get_parent_node};
use hir::ExprKind;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::sym;

// use super::UNNECESSARY_JOIN;

pub(super) fn check<'tcx>(context: &LateContext<'tcx>, expression: &'tcx hir::Expr<'tcx>) {
    // this section should not pass the dogfood test
    // TODO: remove and use a proper test
    let vector = vec!["hello", "world"];
    let output = vector
        .iter()
        .map(|item| item.to_uppercase())
        .collect::<Vec<String>>()
        .join("");
    println!("{}", output);

    // let mut applicability = Applicability::MachineApplicable;

    let parent = get_parent_node(context.tcx, expression.hir_id);

    if parent.is_none() {
        return;
    }

    if_chain! {
        let current_ty = context.typeck_results().expr_ty(expression);
        // the current join method is being called on a vector
        // e.g .join("")
        if is_type_diagnostic_item(context, current_ty, sym::Vec);
        if let Some(parent) = get_parent_expr(context, expression);
        if let ExprKind::MethodCall(_, [self_arg, ..], _) = &parent.kind;
        // the parent collect method is being called on an iterator
        // e.g. .collect<Vec<String>>()
        let caller_ty = context.typeck_results().expr_ty(self_arg);
        if is_type_diagnostic_item(context, caller_ty, sym::Vec);

        // check that the argument for join is an empty string
        // check that the turbofish for collect is <Vec<String>> or <Vec<_>> if the iterator has String items
        then {
            // span_lint_and_sugg(
            //     cx,
            //     UNNECESSARY_JOIN,
            //     span,
            //     &format!(
            //         "called `.collect<Vec<String>>().join("")` on a {1}. Using `.collect::<String>()` is
            // more clear and more concise",          caller_type
            //     ),
            //     "try this",
            //     format!(
            //         "{}.collect::<String>()",
            //         snippet_with_applicability(cx, recv.span, "..", &mut applicability),
            //     ),
            //     applicability,
            // );
            dbg!("{:#?} {:#?}", expression, parent);
            panic!(".collect().join() called on an iterator");
        }
    }
}
