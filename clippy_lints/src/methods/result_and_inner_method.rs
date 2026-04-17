use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::ty::get_adt_inherent_method;
use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::symbol::sym;

use super::RESULT_AND_INNER_METHOD;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>, recv: &'tcx Expr<'_>, arg: &'tcx Expr<'_>) {
    let recv_ty = cx.typeck_results().expr_ty(recv);
    if !recv_ty.is_diag_item(cx, sym::Result) {
        return;
    }
    let ty::Adt(_, args) = recv_ty.kind() else { return };
    let Some(inner_ty) = args.types().next() else { return };
    if get_adt_inherent_method(cx, inner_ty, sym::and).is_none() {
        return;
    }

    let mut applicability = Applicability::MachineApplicable;
    let recv_snip = snippet_with_applicability(cx, recv.span, "..", &mut applicability);
    let arg_snip = snippet_with_applicability(cx, arg.span, "..", &mut applicability);
    span_lint_and_sugg(
        cx,
        RESULT_AND_INNER_METHOD,
        expr.span,
        "called `Result::and` when the inner type also has an `and` method",
        "use fully-qualified syntax to make the intent explicit",
        format!("Result::and({recv_snip}, {arg_snip})"),
        applicability,
    );
}
