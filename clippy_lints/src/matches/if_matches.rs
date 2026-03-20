use super::MATCHES_IF_LET;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::Msrv;
use clippy_utils::source::snippet_with_context;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::needs_ordered_drop;
use clippy_utils::visitors::any_temporaries_need_ordered_drop;
use clippy_utils::{can_use_if_let_chains, contains_name, get_parent_expr, higher, is_expn_of, is_wild, sym};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Arm, Expr, ExprKind};
use rustc_lint::LateContext;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    scrutinee: &'tcx Expr<'tcx>,
    arms: &'tcx [Arm<'tcx>],
    msrv: Msrv,
) {
    let Some(matches_span) = is_expn_of(expr.span, sym::matches) else {
        return;
    };

    let [first_arm, second_arm] = arms else {
        return;
    };

    if !is_matches_expansion(first_arm, second_arm) {
        return;
    }

    let Some(if_expr) = get_parent_expr(cx, expr) else {
        return;
    };

    let ExprKind::If(cond, then, _) = if_expr.kind else {
        return;
    };

    if cond.hir_id != expr.hir_id || if_expr.span.from_expansion() {
        return;
    }

    if needs_ordered_drop(cx, cx.typeck_results().expr_ty(scrutinee))
        || any_temporaries_need_ordered_drop(cx, scrutinee)
    {
        return;
    }

    let mut uses_binding_name_in_body = false;
    first_arm.pat.each_binding_or_first(&mut |_, _, _, ident| {
        uses_binding_name_in_body |= contains_name(ident.name, then, cx);
    });

    if uses_binding_name_in_body {
        return;
    }

    if let Some(guard) = first_arm.guard
        && (!can_use_if_let_chains(cx, msrv) || higher::has_let_expr(guard))
    {
        return;
    }

    let mut app = Applicability::MachineApplicable;
    let ctxt = matches_span.ctxt();
    let pat = snippet_with_context(cx, first_arm.pat.span, ctxt, "..", &mut app).0;
    let scrutinee = Sugg::hir_with_context(cx, scrutinee, ctxt, "..", &mut app)
        .maybe_paren()
        .into_string();

    let suggestion = if let Some(guard) = first_arm.guard {
        let guard = Sugg::hir_with_context(cx, guard, ctxt, "..", &mut app)
            .maybe_paren()
            .into_string();
        format!("let {pat} = {scrutinee} && {guard}")
    } else {
        format!("let {pat} = {scrutinee}")
    };

    span_lint_and_sugg(
        cx,
        MATCHES_IF_LET,
        matches_span,
        "this `matches!` can be written as an `if let`",
        "consider using `if let`",
        suggestion,
        app,
    );
}

fn is_matches_expansion(first_arm: &Arm<'_>, second_arm: &Arm<'_>) -> bool {
    is_arm_bool_lit(first_arm, true) && is_wild(second_arm.pat) && is_arm_bool_lit(second_arm, false)
}

fn is_arm_bool_lit(arm: &Arm<'_>, value: bool) -> bool {
    matches!(
        arm.body.kind,
        ExprKind::Lit(lit) if matches!(lit.node, LitKind::Bool(b) if b == value)
    )
}
