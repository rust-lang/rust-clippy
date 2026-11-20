use std::ops::ControlFlow;

use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::visitors::for_each_local_use_after_expr;
use clippy_utils::{get_expr_use_site, sym};
use rustc_ast::{LitIntType, LitKind};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, GenericArg, HirId, LetStmt, Node, PatKind, PathSegment};
use rustc_lint::LateContext;

use crate::methods::method_call;

use super::STR_PARSE_INFER_TO_I32;

pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
    // first check thet 'parse' method is actually have the inherent str.parse impl
    if let Some(call_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
        && let Some(impl_id) = cx.tcx.inherent_impl_of_assoc(call_id)
        && let self_ty = cx.tcx.type_of(impl_id).instantiate_identity().skip_norm_wip()
        && self_ty.is_str()
        // check result type is i32
        && let result_ty = cx.typeck_results().node_args(expr.hir_id)
            && matches!(
                result_ty.type_at(0).kind(),
                rustc_middle::ty::Int(rustc_middle::ty::IntTy::I32)
            )
        // Now we check if the i32 was explicit
        && let ExprKind::MethodCall(parse_segment, ..) = expr.kind
        && !is_turbofish_constrained(parse_segment)
        && !is_constrained(cx, expr)
    {
        // `parse` alone, or `parse::<_>` if the user wrote an inferred turbofish
        let parse_span = match parse_segment.args {
            Some(generic_args) => parse_segment.ident.span.to(generic_args.span_ext),
            None => parse_segment.ident.span,
        };

        span_lint_and_then(
            cx,
            STR_PARSE_INFER_TO_I32,
            expr.span,
            "the type parsed by `str::parse` silently falls back to `i32`",
            |diag| {
                diag.span_suggestion_verbose(
                    parse_span,
                    "if `i32` is intended, specify it explicitly",
                    "parse::<i32>",
                    Applicability::MaybeIncorrect,
                );
            },
        );
    }
}

fn is_turbofish_constrained(segment: &PathSegment<'_>) -> bool {
    let Some(gen_args) = segment.args else {
        return false; // no generic args
    };
    gen_args.args.iter().any(|arg| !matches!(arg, GenericArg::Infer(_)))
}

// Walk outward from parse call looking for type constraints
fn is_constrained<'tcx>(cx: &LateContext<'tcx>, parse_call: &'tcx Expr<'tcx>) -> bool {
    let mut current = parse_call;
    loop {
        let use_site = get_expr_use_site(cx.tcx, cx.typeck_results(), current.span.ctxt(), current);
        if !use_site.same_ctxt {
            // avoid entering a macro context
            return true;
        }
        let Node::Expr(parent) = use_site.node else {
            return match use_site.node {
                Node::LetStmt(LetStmt {
                    ty: None, /* no `let` type constraint */
                    pat,
                    ..
                }) => match pat.kind {
                    PatKind::Binding(_, local_id, _, None) => is_any_use_of_local_constrained(cx, local_id, current),
                    PatKind::Wild => false, // `let _ = ..` nothing can constrain value
                    _ => true,              // destructuring, `x @ pat`, ...: not handled
                },
                _ => true,
            };
        };

        match parent.kind {
            ExprKind::MethodCall(..) => {
                // `method_call` return `None` if the receiver or args come from a macro
                let Some((name, receiver, args, _, _)) = method_call(parent) else {
                    return true;
                };
                if receiver.hir_id != use_site.child_id {
                    return true;
                }
                let passes_type_through = match (name, args) {
                    (sym::ok | sym::unwrap | sym::unwrap_or_default, []) | (sym::expect, [_]) => true,
                    (sym::unwrap_or, [default]) => is_unsuffixed_int_lit(default),
                    _ => false,
                };
                if !passes_type_through {
                    return true;
                }
                current = parent; // keep following the unwrapped value outward
            },
            ExprKind::Binary(op, lhs, rhs) if op.node.is_comparison() => {
                let other_operand = if lhs.hir_id == use_site.child_id { rhs } else { lhs };
                return !is_unsuffixed_int_lit(other_operand);
            },
            ExprKind::Cast(..) => return false,
            _ => return true,
        }
    }
}

fn is_unsuffixed_int_lit(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Lit(lit) => matches!(lit.node, LitKind::Int(_, LitIntType::Unsuffixed)),
        ExprKind::Unary(rustc_ast::UnOp::Neg | rustc_ast::UnOp::Not, inner_expr) => is_unsuffixed_int_lit(inner_expr),
        _ => false,
    }
}

fn is_any_use_of_local_constrained<'tcx>(cx: &LateContext<'tcx>, local_id: HirId, init: &'tcx Expr<'tcx>) -> bool {
    for_each_local_use_after_expr(cx, local_id, init.hir_id, |use_of_local| {
        if is_constrained(cx, use_of_local) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_break()
}
