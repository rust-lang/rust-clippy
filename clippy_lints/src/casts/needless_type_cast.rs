use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::visitors::for_each_expr_without_closures;
use core::ops::ControlFlow;
use rustc_data_structures::fx::FxHashMap;
use rustc_errors::Applicability;
use rustc_hir::def::Res;
use rustc_hir::{Block, Body, ExprKind, HirId, LetStmt, PatKind, StmtKind};
use rustc_lint::LateContext;
use rustc_middle::ty::Ty;
use rustc_span::Span;

use super::NEEDLESS_TYPE_CAST;

struct BindingInfo<'a> {
    source_ty: Ty<'a>,
    ty_span: Option<Span>,
    pat_span: Span,
}

struct UsageInfo<'a> {
    is_cast: bool,
    cast_to: Option<Ty<'a>>,
}

pub(super) fn check<'a>(cx: &LateContext<'a>, body: &Body<'a>) {
    let mut bindings: FxHashMap<HirId, BindingInfo<'a>> = FxHashMap::default();

    collect_bindings_from_block(cx, body.value, &mut bindings);

    for_each_expr_without_closures(body.value, |expr| {
        if let ExprKind::Let(let_expr) = expr.kind {
            collect_binding_from_let(cx, let_expr, &mut bindings);
        }
        ControlFlow::<()>::Continue(())
    });

    #[allow(rustc::potential_query_instability)]
    let mut binding_vec: Vec<_> = bindings.into_iter().collect();
    binding_vec.sort_by_key(|(_, info)| info.pat_span.lo());

    for (hir_id, binding_info) in binding_vec {
        check_binding_usages(cx, body, hir_id, &binding_info);
    }
}

fn collect_bindings_from_block<'a>(
    cx: &LateContext<'a>,
    expr: &rustc_hir::Expr<'a>,
    bindings: &mut FxHashMap<HirId, BindingInfo<'a>>,
) {
    if let ExprKind::Block(block, _) = expr.kind {
        collect_bindings_from_block_inner(cx, block, bindings);
    }
}

fn collect_bindings_from_block_inner<'a>(
    cx: &LateContext<'a>,
    block: &Block<'a>,
    bindings: &mut FxHashMap<HirId, BindingInfo<'a>>,
) {
    for stmt in block.stmts {
        if let StmtKind::Let(let_stmt) = stmt.kind {
            collect_binding_from_local(cx, let_stmt, bindings);
        }
    }
}

fn collect_binding_from_let<'a>(
    cx: &LateContext<'a>,
    let_expr: &rustc_hir::LetExpr<'a>,
    bindings: &mut FxHashMap<HirId, BindingInfo<'a>>,
) {
    if let_expr.ty.is_none() {
        return;
    }

    if let PatKind::Binding(_, hir_id, _, _) = let_expr.pat.kind {
        let ty = cx.typeck_results().pat_ty(let_expr.pat);
        if ty.is_numeric() {
            bindings.insert(
                hir_id,
                BindingInfo {
                    source_ty: ty,
                    ty_span: let_expr.ty.map(|t| t.span),
                    pat_span: let_expr.pat.span,
                },
            );
        }
    }
}

fn collect_binding_from_local<'a>(
    cx: &LateContext<'a>,
    let_stmt: &LetStmt<'a>,
    bindings: &mut FxHashMap<HirId, BindingInfo<'a>>,
) {
    // Only check bindings with explicit type annotations
    // Otherwise, the suggestion to change the type may not be valid
    // (e.g., `let x = 42u8;` cannot just change to `let x: i64 = 42u8;`)
    if let_stmt.ty.is_none() {
        return;
    }

    if let PatKind::Binding(_, hir_id, _, _) = let_stmt.pat.kind {
        let ty = cx.typeck_results().pat_ty(let_stmt.pat);
        if ty.is_numeric() {
            bindings.insert(
                hir_id,
                BindingInfo {
                    source_ty: ty,
                    ty_span: let_stmt.ty.map(|t| t.span),
                    pat_span: let_stmt.pat.span,
                },
            );
        }
    }
}

fn check_binding_usages<'a>(cx: &LateContext<'a>, body: &Body<'a>, hir_id: HirId, binding_info: &BindingInfo<'a>) {
    let mut usages: Vec<UsageInfo<'a>> = Vec::new();

    for_each_expr_without_closures(body.value, |expr| {
        if let ExprKind::Path(ref qpath) = expr.kind
            && let Res::Local(id) = cx.qpath_res(qpath, expr.hir_id)
            && id == hir_id
        {
            let parent_id = cx.tcx.parent_hir_id(expr.hir_id);
            let parent = cx.tcx.hir_node(parent_id);

            if let rustc_hir::Node::Expr(parent_expr) = parent {
                if let ExprKind::Cast(_, _) = parent_expr.kind {
                    let target_ty = cx.typeck_results().expr_ty(parent_expr);
                    usages.push(UsageInfo {
                        is_cast: true,
                        cast_to: Some(target_ty),
                    });
                } else {
                    usages.push(UsageInfo {
                        is_cast: false,
                        cast_to: None,
                    });
                }
            } else {
                usages.push(UsageInfo {
                    is_cast: false,
                    cast_to: None,
                });
            }
        }
        ControlFlow::<()>::Continue(())
    });

    if usages.is_empty() {
        return;
    }

    if !usages.iter().all(|u| u.is_cast) {
        return;
    }

    let Some(first_target) = usages.first().and_then(|u| u.cast_to) else {
        return;
    };

    if !usages.iter().all(|u| u.cast_to == Some(first_target)) {
        return;
    }

    if first_target == binding_info.source_ty {
        return;
    }

    let suggestion = if binding_info.ty_span.is_some() {
        format!("{first_target}")
    } else {
        format!(": {first_target}")
    };

    let span = binding_info.ty_span.unwrap_or(binding_info.pat_span);
    let current_snippet = snippet(cx, span, "_");

    span_lint_and_sugg(
        cx,
        NEEDLESS_TYPE_CAST,
        span,
        format!(
            "this binding is defined as `{}` but is always cast to `{}`",
            binding_info.source_ty, first_target
        ),
        "consider defining it as",
        if binding_info.ty_span.is_some() {
            suggestion
        } else {
            format!("{current_snippet}{suggestion}")
        },
        Applicability::MaybeIncorrect,
    );
}
