use super::MATCHES_WITH_UNRELATED_IF;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_context;
use clippy_utils::sugg::Sugg;
use clippy_utils::visitors::is_local_used;
use clippy_utils::{get_parent_expr, sugg};
use rustc_errors::Applicability;
use rustc_hir::{Arm, BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::Span;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    ex: &'tcx Expr<'tcx>,
    arm: &'tcx Arm<'tcx>,
    call_site: Span,
) {
    if call_site.from_expansion() {
        return;
    }

    let Some(guard) = arm.guard else {
        return;
    };

    if guard_uses_binding(cx, arm, guard) {
        return;
    }

    let parent = get_parent_expr(cx, expr);

    // `matches!(.. if ..) || matches!(.. if ..)` reads better with the guards grouped
    // inside their `matches!`, so don't suggest hoisting them out.
    if let Some(parent) = parent
        && let ExprKind::Binary(op, ..) = parent.kind
        && op.node == BinOpKind::Or
    {
        return;
    }
    let mut app = Applicability::MachineApplicable;
    let ctxt = call_site.ctxt();

    let scrutinee = snippet_with_context(cx, ex.span, ctxt, "..", &mut app).0;
    let pat = snippet_with_context(cx, arm.pat.span, ctxt, "..", &mut app).0;

    let lhs = Sugg::NonParen(format!("matches!({scrutinee}, {pat})").into());
    let rhs = Sugg::hir_with_context(cx, guard, ctxt, "..", &mut app);

    // parenthesizes `rhs` when needed (e.g. `a || b`).
    let mut sugg = sugg::make_binop(BinOpKind::And, &lhs, &rhs);

    // wrap it if the parent binds tighter.
    if parent.is_some_and(|p| parent_binds_tighter_than_and(expr, p)) {
        sugg = sugg.maybe_paren();
    }

    span_lint_and_sugg(
        cx,
        MATCHES_WITH_UNRELATED_IF,
        call_site,
        "this `if` guard doesn't use any variable bound by the pattern",
        "move the guard outside of `matches!`",
        sugg.to_string(),
        app,
    );
}

fn guard_uses_binding<'tcx>(cx: &LateContext<'tcx>, arm: &Arm<'tcx>, guard: &'tcx Expr<'tcx>) -> bool {
    let mut found = false;
    arm.pat.each_binding(|_, hir_id, _, _| {
        found |= is_local_used(cx, guard, hir_id);
    });

    found
}

fn parent_binds_tighter_than_and(expr: &Expr<'_>, parent: &Expr<'_>) -> bool {
    match parent.kind {
        ExprKind::Unary(..) | ExprKind::AddrOf(..) | ExprKind::Cast(..) | ExprKind::Field(..) => true,
        ExprKind::MethodCall(_, recv, ..) => recv.hir_id == expr.hir_id,
        ExprKind::Index(base, ..) => base.hir_id == expr.hir_id,
        ExprKind::Binary(op, ..) => op.node != BinOpKind::And,
        _ => false,
    }
}
