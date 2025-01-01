use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::source::snippet;
use clippy_utils::ty::get_type_diagnostic_name;
use rustc_ast::ast::UnOp;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::{Symbol, sym};

use super::CLONE_ON_ARC_OR_RC;

pub(super) fn check(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    method_name: Symbol,
    receiver: &Expr<'_>,
    args: &[Expr<'_>],
    msrv: &Msrv,
) {
    if !msrv.meets(msrvs::ARC_RC_UNWRAP_OR_CLONE) {
        return;
    }

    if method_name == sym::clone
        && args.is_empty()
        && let ExprKind::Unary(UnOp::Deref, recv) = receiver.kind
        && let Some(arc_or_rc_path) = is_arc_or_rc(cx, recv)
    {
        span_lint_and_sugg(
            cx,
            CLONE_ON_ARC_OR_RC,
            expr.span,
            "conditionally unwrapping an `Arc`/`Rc` may avoid unnecessary clone",
            "try",
            format!(
                "{arc_or_rc_path}::unwrap_or_clone({snip})",
                snip = snippet(cx, recv.span, "..")
            ),
            Applicability::MachineApplicable,
        );
    }
}

fn is_arc_or_rc(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<&'static str> {
    match get_type_diagnostic_name(cx, cx.typeck_results().expr_ty(expr)) {
        Some(sym::Arc) => Some("std::sync::Arc"),
        Some(sym::Rc) => Some("std::rc::Rc"),
        _ => None,
    }
}
