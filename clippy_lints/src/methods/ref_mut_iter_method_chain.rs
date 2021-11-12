use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{snippet_expr, TargetPrecedence};
use clippy_utils::ty::implements_trait;
use if_chain::if_chain;
use rustc_errors::Applicability;
use rustc_hir::{BorrowKind, Expr, ExprKind, Mutability, UnOp};
use rustc_lint::LateContext;
use rustc_span::sym;

use super::REF_MUT_ITER_METHOD_CHAIN;

pub(crate) fn check(cx: &LateContext<'_>, self_arg: &Expr<'_>) {
    if_chain! {
        if let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, base_expr) = self_arg.kind;
        if !self_arg.span.from_expansion();
        if let Some(&iter_trait) = cx.tcx.all_diagnostic_items(()).name_to_id.get(&sym::Iterator);
        if implements_trait(cx, cx.typeck_results().expr_ty(base_expr).peel_refs(), iter_trait, &[]);
        then {
            let snip_expr = match base_expr.kind {
                ExprKind::Unary(UnOp::Deref, e) if cx.typeck_results().expr_ty(e).is_ref() && !base_expr.span.from_expansion()
                    => e,
                _ => base_expr,
            };
            let mut app = Applicability::MachineApplicable;
            span_lint_and_sugg(
                cx,
                REF_MUT_ITER_METHOD_CHAIN,
                self_arg.span,
                "use of `&mut` on an iterator in a method chain",
                "try",
                format!(
                    "{}.by_ref()",
                    snippet_expr(cx, snip_expr, TargetPrecedence::Postfix, self_arg.span.ctxt(), &mut app),
                ),
                app,
            );
        }
    }
}
