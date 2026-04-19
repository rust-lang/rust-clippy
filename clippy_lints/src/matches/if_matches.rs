use super::MATCHES_IF_LET;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::Msrv;
use clippy_utils::res::MaybeResPath;
use clippy_utils::source::snippet_with_context;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::needs_ordered_drop;
use clippy_utils::visitors::any_temporaries_need_ordered_drop;
use clippy_utils::{can_use_if_let_chains, higher};
use rustc_data_structures::sso::SsoHashSet;
use rustc_errors::Applicability;
use rustc_hir::intravisit::{Visitor, walk_expr};
use rustc_hir::{Arm, BinOpKind, Expr, ExprKind, Node};
use rustc_lint::LateContext;
use rustc_middle::hir::nested_filter;
use rustc_span::{Span, Symbol, SyntaxContext};
use std::ops::ControlFlow;

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    matches_span: Span,
    scrutinee: &'tcx Expr<'tcx>,
    arm: &'tcx Arm<'tcx>,
    msrv: Msrv,
) {
    let Some(EnclosingIf {
        then,
        needs_if_let_chain,
    }) = find_enclosing_if(cx, expr, matches_span.ctxt())
    else {
        return;
    };

    if needs_ordered_drop(cx, cx.typeck_results().expr_ty(scrutinee))
        || any_temporaries_need_ordered_drop(cx, scrutinee)
    {
        return;
    }

    if body_uses_binding_name(cx, arm, then) {
        return;
    }

    if (needs_if_let_chain || arm.guard.is_some()) && !can_use_if_let_chains(cx, msrv) {
        return;
    }

    if let Some(guard) = arm.guard
        && higher::has_let_expr(guard)
    {
        return;
    }

    let mut app = Applicability::MachineApplicable;
    let ctxt = matches_span.ctxt();
    let pat = snippet_with_context(cx, arm.pat.span, ctxt, "..", &mut app).0;
    let scrutinee = Sugg::hir_with_context(cx, scrutinee, ctxt, "..", &mut app).maybe_paren();

    let suggestion = if let Some(guard) = arm.guard {
        let guard = Sugg::hir_with_context(cx, guard, ctxt, "..", &mut app).maybe_paren();
        format!("let {pat} = {scrutinee} && {guard}")
    } else {
        format!("let {pat} = {scrutinee}")
    };

    span_lint_and_sugg(
        cx,
        MATCHES_IF_LET,
        matches_span,
        "`matches!` used as an `if` condition",
        "use `if let`",
        suggestion,
        app,
    );
}

struct EnclosingIf<'tcx> {
    then: &'tcx Expr<'tcx>,
    needs_if_let_chain: bool,
}

fn find_enclosing_if<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    ctxt: SyntaxContext,
) -> Option<EnclosingIf<'tcx>> {
    let mut child_id = expr.hir_id;
    let mut needs_if_let_chain = false;

    for (parent_id, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        let Node::Expr(parent_expr) = node else {
            return None;
        };

        if parent_expr.span.ctxt() != ctxt {
            return None;
        }

        match parent_expr.kind {
            ExprKind::Binary(op, left, right)
                if op.node == BinOpKind::And && (left.hir_id == child_id || right.hir_id == child_id) =>
            {
                child_id = parent_id;
                needs_if_let_chain = true;
            },
            ExprKind::DropTemps(inner) if inner.hir_id == child_id => child_id = parent_id,
            ExprKind::If(cond, then, _) if cond.hir_id == child_id => {
                return Some(EnclosingIf {
                    then,
                    needs_if_let_chain,
                });
            },
            _ => return None,
        }
    }

    None
}

fn body_uses_binding_name<'tcx>(cx: &LateContext<'tcx>, arm: &Arm<'tcx>, then: &'tcx Expr<'tcx>) -> bool {
    let mut binding_names = SsoHashSet::default();
    arm.pat.each_binding_or_first(&mut |_, _, _, ident| {
        binding_names.insert(ident.name);
    });

    if binding_names.is_empty() {
        return false;
    }

    let mut visitor = BodyUsesBindingName { cx, binding_names };
    visitor.visit_expr(then).is_break()
}

struct BodyUsesBindingName<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    binding_names: SsoHashSet<Symbol>,
}

impl<'tcx> Visitor<'tcx> for BodyUsesBindingName<'_, 'tcx> {
    type Result = ControlFlow<()>;
    type NestedFilter = nested_filter::OnlyBodies;

    fn visit_expr(&mut self, expr: &'tcx Expr<'_>) -> Self::Result {
        if expr
            .res_local_id()
            .is_some_and(|id| self.binding_names.contains(&self.cx.tcx.hir_name(id)))
        {
            ControlFlow::Break(())
        } else {
            walk_expr(self, expr)
        }
    }

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }
}
