use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_middle::ty;

use super::UNNECESSARY_TO_OWNED;

pub(super) fn check(cx: &LateContext<'_>, expr: &Expr<'_>, recv: &Expr<'_>, arg: &Expr<'_>) {
    let recv_ty = cx.typeck_results().expr_ty(recv).peel_refs();
    if !recv_ty.is_diag_item(cx, sym::Vec) {
        return;
    }

    if let ExprKind::MethodCall(method, src_recv, [], _) = &arg.kind
        && matches!(method.ident.name, sym::to_vec | sym::to_owned | sym::clone)
    {
        let src_ty = cx.typeck_results().expr_ty(src_recv);
        let src_ty_peeled = src_ty.peel_refs();

        // Check src_typ is a slice type (either &[T] or &Vec<T> which derefs to &[T])
        let is_slice = matches!(src_ty_peeled.kind(), ty::Slice(..)) || src_ty_peeled.is_diag_item(cx, sym::Vec);

        // Skip self-extend: vec.extend(vec.to_vec())
        if is_slice && !eq_expr_value(cx, recv, src_recv) {
            let mut applicability = Applicability::MachineApplicable;
            let recv_snip = snippet_with_applicability(cx, recv.span, "..", &mut applicability);
            let src_snip = snippet_with_applicability(cx, src_recv.span, "..", &mut applicability);
            span_lint_and_sugg(
                cx,
                UNNECESSARY_TO_OWNED,
                expr.span,
                "unnecessary allocation in argument to `extend`",
                "try",
                format!("{recv_snip}.extend_from_slice({src_snip})"),
                applicability,
            );
        }
    }
}
