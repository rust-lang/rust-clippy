use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet_with_context;
use clippy_utils::ty::implements_trait;
use clippy_utils::{get_parent_expr, is_diag_item_method, is_diag_trait_item, peel_middle_ty_refs, sym};
use rustc_errors::Applicability;
use rustc_hir as hir;
use rustc_lint::LateContext;
use rustc_span::Symbol;
use std::ops::ControlFlow;

use super::IMPLICIT_CLONE;

pub fn check(cx: &LateContext<'_>, method_name: Symbol, expr: &hir::Expr<'_>, recv: &hir::Expr<'_>) {
    if let Some(method_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && is_clone_like(cx, method_name, method_def_id)
        && let return_type = cx.typeck_results().expr_ty(expr)
        && let input_type = cx.typeck_results().expr_ty(recv)
        && let (input_type, ref_count) = peel_middle_ty_refs(input_type)
        && !(ref_count > 0 && is_diag_trait_item(cx, method_def_id, sym::ToOwned))
        && let Some(ty_name) = input_type.ty_adt_def().map(|adt_def| cx.tcx.item_name(adt_def.did()))
        && return_type == input_type
        && let Some(clone_trait) = cx.tcx.lang_items().clone_trait()
        && implements_trait(cx, return_type, clone_trait, &[])
        && is_called_from_map_like(cx, expr).is_none()
    {
        let mut app = Applicability::MachineApplicable;
        let recv_snip = snippet_with_context(cx, recv.span, expr.span.ctxt(), "..", &mut app).0;
        span_lint_and_sugg(
            cx,
            IMPLICIT_CLONE,
            expr.span,
            format!("implicitly cloning a `{ty_name}` by calling `{method_name}` on its dereferenced type"),
            "consider using",
            if ref_count > 1 {
                format!("({}{recv_snip}).clone()", "*".repeat(ref_count - 1))
            } else {
                format!("{recv_snip}.clone()")
            },
            app,
        );
    }
}

/// Returns true if the named method can be used to clone the receiver.
pub fn is_clone_like(cx: &LateContext<'_>, method_name: Symbol, method_def_id: hir::def_id::DefId) -> bool {
    match method_name {
        sym::to_os_string => is_diag_item_method(cx, method_def_id, sym::OsStr),
        sym::to_owned => is_diag_trait_item(cx, method_def_id, sym::ToOwned),
        sym::to_path_buf => is_diag_item_method(cx, method_def_id, sym::Path),
        sym::to_string => is_diag_trait_item(cx, method_def_id, sym::ToString),
        sym::to_vec => cx
            .tcx
            .impl_of_method(method_def_id)
            .filter(|&impl_did| {
                cx.tcx.type_of(impl_did).instantiate_identity().is_slice() && cx.tcx.impl_trait_ref(impl_did).is_none()
            })
            .is_some(),
        _ => false,
    }
}

fn is_called_from_map_like(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> Option<rustc_span::Span> {
    // Look for a closure as parent of `expr`, discarding simple blocks
    let parent_closure = cx
        .tcx
        .hir_parent_iter(expr.hir_id)
        .try_fold(expr.hir_id, |child_hir_id, (_, node)| match node {
            // Check that the child expression is the only expression in the block
            hir::Node::Block(block) if block.stmts.is_empty() && block.expr.map(|e| e.hir_id) == Some(child_hir_id) => {
                ControlFlow::Continue(block.hir_id)
            },
            hir::Node::Expr(expr) if matches!(expr.kind, hir::ExprKind::Block(..)) => {
                ControlFlow::Continue(expr.hir_id)
            },
            hir::Node::Expr(expr) if matches!(expr.kind, hir::ExprKind::Closure(_)) => ControlFlow::Break(Some(expr)),
            _ => ControlFlow::Break(None),
        })
        .break_value()?;
    is_parent_map_like(cx, parent_closure?)
}

fn is_parent_map_like(cx: &LateContext<'_>, expr: &hir::Expr<'_>) -> Option<rustc_span::Span> {
    if let Some(parent_expr) = get_parent_expr(cx, expr)
        && let hir::ExprKind::MethodCall(name, _, _, parent_span) = parent_expr.kind
        && name.ident.name == sym::map
        && let Some(caller_def_id) = cx.typeck_results().type_dependent_def_id(parent_expr.hir_id)
        && (is_diag_item_method(cx, caller_def_id, sym::Result)
            || is_diag_item_method(cx, caller_def_id, sym::Option)
            || is_diag_trait_item(cx, caller_def_id, sym::Iterator))
    {
        Some(parent_span)
    } else {
        None
    }
}
