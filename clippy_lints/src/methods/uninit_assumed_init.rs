use clippy_utils::diagnostics::span_lint;
use clippy_utils::is_item;
use if_chain::if_chain;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};
use rustc_span::sym;

use super::UNINIT_ASSUMED_INIT;

/// lint for `MaybeUninit::uninit().assume_init()` (we already have the latter)
pub(super) fn check(cx: &LateContext<'_>, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>) {
    if_chain! {
        if let hir::ExprKind::Call(callee, args) = recv.kind;
        if args.is_empty();
        if is_item(cx, callee, sym::maybe_uninit_uninit);
        if !is_maybe_uninit_ty_valid(cx, cx.typeck_results().expr_ty_adjusted(expr));
        then {
            span_lint(
                cx,
                UNINIT_ASSUMED_INIT,
                expr.span,
                "this call for this type may be undefined behavior"
            );
        }
    }
}

fn is_maybe_uninit_ty_valid(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Array(component, _) => is_maybe_uninit_ty_valid(cx, component),
        ty::Tuple(types) => types.types().all(|ty| is_maybe_uninit_ty_valid(cx, ty)),
        ty::Adt(adt, _) => is_item(cx, adt.did, hir::LangItem::MaybeUninit),
        _ => false,
    }
}
