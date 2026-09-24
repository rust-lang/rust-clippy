use clippy_utils::diagnostics::span_lint;
use clippy_utils::ty::is_c_void;
use clippy_utils::usage::local_used_in;
use clippy_utils::visitors::for_each_local_use_after_expr;
use clippy_utils::{get_parent_expr, is_hir_ty_cfg_dependant, is_never_expr, sym};
use rustc_hir::{BinOpKind, BindingMode, Expr, ExprKind, GenericArg, HirId, Node, PatKind, UnOp};
use rustc_lint::LateContext;
use rustc_middle::ty::layout::LayoutOf as _;
use rustc_middle::ty::{self, Ty};
use std::ops::ControlFlow;

use super::CAST_PTR_ALIGNMENT;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>, cast_from: Ty<'tcx>, cast_to: Ty<'tcx>) {
    lint_cast_ptr_alignment(cx, expr, cast_from, cast_to);
}

pub(super) fn check_cast_method(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::MethodCall(method_path, self_arg, [], _) = &expr.kind
        && method_path.ident.name == sym::cast
        && let Some(generic_args) = method_path.args
        && let [GenericArg::Type(cast_to)] = generic_args.args
        // There probably is no obvious reason to do this, just to be consistent with `as` cases.
        && !is_hir_ty_cfg_dependant(cx, cast_to.as_unambig_ty())
    {
        let (cast_from, cast_to) = (cx.typeck_results().expr_ty(self_arg), cx.typeck_results().expr_ty(expr));
        lint_cast_ptr_alignment(cx, expr, cast_from, cast_to);
    }
}

fn lint_cast_ptr_alignment<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'_>, cast_from: Ty<'tcx>, cast_to: Ty<'tcx>) {
    if let ty::RawPtr(from_ptr_ty, _) = *cast_from.kind()
        && let ty::RawPtr(to_ptr_ty, _) = *cast_to.kind()
        && let Ok(from_layout) = cx.layout_of(from_ptr_ty)
        && let Ok(to_layout) = cx.layout_of(to_ptr_ty)
        && from_layout.align.abi < to_layout.align.abi
        // with c_void, we inherently need to trust the user
        && !is_c_void(cx, from_ptr_ty)
        // when casting from a ZST, we don't know enough to properly lint
        && !from_layout.is_zst()
        && !is_used_safely(cx, expr)
        && !is_guarded_by_alignment_check(cx, expr)
        && !expr.span.in_external_macro(cx.tcx.sess.source_map())
    {
        span_lint(
            cx,
            CAST_PTR_ALIGNMENT,
            expr.span,
            format!(
                "casting from `{cast_from}` to a more-strictly-aligned pointer (`{cast_to}`) ({} < {} bytes)",
                from_layout.align.bytes(),
                to_layout.align.bytes(),
            ),
        );
    }
}

fn is_used_safely(cx: &LateContext<'_>, e: &Expr<'_>) -> bool {
    let Some(parent) = get_parent_expr(cx, e) else {
        return false;
    };
    match parent.kind {
        ExprKind::MethodCall(name, self_arg, ..) if self_arg.hir_id == e.hir_id => {
            matches!(
                name.ident.name,
                sym::read_unaligned | sym::write_unaligned | sym::is_aligned
            ) && is_raw_ptr_method(cx, parent)
        },
        ExprKind::Call(func, [arg, ..]) if arg.hir_id == e.hir_id => {
            if let ExprKind::Path(path) = &func.kind
                && let Some(def_id) = cx.qpath_res(path, func.hir_id).opt_def_id()
                && let Some(name) = cx.tcx.get_diagnostic_name(def_id)
            {
                matches!(
                    name,
                    sym::ptr_write_unaligned
                        | sym::ptr_read_unaligned
                        | sym::intrinsics_unaligned_volatile_load
                        | sym::intrinsics_unaligned_volatile_store
                )
            } else {
                false
            }
        },
        _ => false,
    }
}

fn is_raw_ptr_method(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    let Some(def_id) = cx.tcx.impl_of_assoc(def_id) else {
        return false;
    };

    cx.tcx
        .type_of(def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .is_raw_ptr()
}

// TODO: this is a purely syntactic check and is not exhaustive. It only recognizes an
// `is_aligned()` call as the pointer's very first use, inside a diverging `if` sitting directly in
// the block the pointer is bound in. Guards hidden behind a helper function, a `match`, a stored
// `bool`, or a conditionally executed block are missed. This should be replaced once `clippy_utils`
// grows a proper "is this pointer known to be aligned here" query.
fn is_guarded_by_alignment_check(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let Node::LetStmt(local) = cx.tcx.parent_hir_node(expr.hir_id)
        && let PatKind::Binding(BindingMode::NONE, local_id, _, None) = local.pat.kind
        && local.init.is_some_and(|init| init.hir_id == expr.hir_id)
        && let Some(block_id) = enclosing_block_id(cx, local.hir_id)
        && let ControlFlow::Break(first_use) =
            for_each_local_use_after_expr(cx, local_id, expr.hir_id, ControlFlow::Break)
        && let Some(check) = get_parent_expr(cx, first_use)
        && is_alignment_check(cx, check)
        && let Some(negation) = get_parent_expr(cx, check)
        && matches!(negation.kind, ExprKind::Unary(UnOp::Not, _))
        && let Some(guard) = peel_or_chain(cx, negation)
        && let ExprKind::If(_, then, None) = guard.kind
        && enclosing_block_id(cx, guard.hir_id) == Some(block_id)
        && is_never_expr(cx, then).is_some()
        && !local_used_in(cx, local_id, then)
    {
        true
    } else {
        false
    }
}

fn enclosing_block_id(cx: &LateContext<'_>, hir_id: HirId) -> Option<HirId> {
    let Node::Stmt(stmt) = cx.tcx.parent_hir_node(hir_id) else {
        return None;
    };
    match cx.tcx.parent_hir_node(stmt.hir_id) {
        Node::Block(block) => Some(block.hir_id),
        _ => None,
    }
}

fn peel_or_chain<'tcx>(cx: &LateContext<'tcx>, mut expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    loop {
        let parent = get_parent_expr(cx, expr)?;
        if matches!(parent.kind, ExprKind::Binary(op, ..) if op.node == BinOpKind::Or) {
            expr = parent;
        } else {
            return Some(parent);
        }
    }
}

fn is_alignment_check(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(
        expr.kind,
        ExprKind::MethodCall(name, _, [], _)
            if name.ident.name == sym::is_aligned && is_raw_ptr_method(cx, expr)
    )
}
