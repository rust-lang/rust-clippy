use super::MANUAL_MEMCPY;
use crate::loops::manual_memcpy::{
    apply_offset, get_assignment, get_assignments, get_details_from_idx, get_loop_counters, get_slice_like_element_ty,
    IndexExpr, MinifyingSugg, Start, StartKind,
};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::is_copy;
use clippy_utils::{higher, path_to_local, sugg};
use if_chain::if_chain;
use rustc_ast::ast;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, Pat, PatKind};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::symbol::sym;
use std::iter::Iterator;

/// Checks for for loops that sequentially copy items within a slice
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'_>,
    arg: &'tcx Expr<'_>,
    body: &'tcx Expr<'_>,
    expr: &'tcx Expr<'_>,
) -> bool {
    if let Some(higher::Range {
        start: Some(start),
        end: Some(end),
        limits,
    }) = higher::Range::hir(arg)
    {
        // the var must be a single name
        if let PatKind::Binding(_, canonical_id, _, _) = pat.kind {
            let starts = &[Start {
                id: canonical_id,
                kind: StartKind::Range,
            }];

            // This is one of few ways to return different iterators
            // derived from: https://stackoverflow.com/questions/29760668/conditionally-iterate-over-one-of-several-possible-iterators/52064434#52064434
            let mut iter_a = None;
            let mut iter_b = None;

            if let ExprKind::Block(block, _) = body.kind {
                if let Some(mut loop_counters) = get_loop_counters(cx, block, expr) {
                    // we currently do not support loop counters at all, as we would need to know
                    // their initial value to assess whether the copy is safe to do (same reason we
                    // require positive offsets in source)
                    if loop_counters.next().is_some() {
                        return false;
                    }
                }
                iter_a = Some(get_assignments(block, starts));
            } else {
                iter_b = Some(get_assignment(body));
            }

            let assignments = iter_a.into_iter().flatten().chain(iter_b.into_iter());

            let big_sugg = assignments
                // The only statements in the for loops can be indexed assignments from
                // indexed retrievals (except increments of loop counters).
                .map(|o| {
                    o.and_then(|(lhs, rhs)| {
                        if_chain! {
                            if let ExprKind::Index(base_left, idx_left) = lhs.kind;
                            if let ExprKind::Index(base_right, idx_right) = rhs.kind;
                            // Source and destination must be same
                            if let Some(base_left_local) = path_to_local(base_left);
                            if let Some(base_right_local) = path_to_local(base_right);
                            if base_left_local == base_right_local;
                            if let Some(ty) = get_slice_like_element_ty(cx, cx.typeck_results().expr_ty(base_left));
                            if let Some((start_left, offset_left)) = get_details_from_idx(cx, idx_left, starts);
                            if let Some((start_right, offset_right)) = get_details_from_idx(cx, idx_right, starts);

                            if left_is_smaller_than_right(cx, idx_left, idx_right);

                            if let StartKind::Range = start_left;
                            if let StartKind::Range = start_right;

                            if is_copy(cx, ty);

                            then {
                                Some((IndexExpr { base: base_left, idx: start_left, idx_offset: offset_left },
                                        IndexExpr { base: base_right, idx: start_right, idx_offset: offset_right }))
                            } else {
                                None
                            }
                        }
                    })
                })
                .map(|o| o.map(|(dst, src)| build_manual_memmove_suggestion(cx, start, end, limits, &dst, &src)))
                .collect::<Option<Vec<_>>>()
                .filter(|v| {
                    // we currently do not support more than one assignment, as it's "too hard" to
                    // prove that the to-be-moved slices (+ their destinations) are nonoverlapping
                    v.len() == 1
                })
                .map(|v| v.join("\n    "));

            if let Some(big_sugg) = big_sugg {
                span_lint_and_sugg(
                    cx,
                    MANUAL_MEMCPY,
                    expr.span,
                    "it looks like you're manually copying within a slice",
                    "try replacing the loop by",
                    big_sugg,
                    Applicability::Unspecified,
                );
                return true;
            }
        }
    }
    false
}

fn build_manual_memmove_suggestion<'tcx>(
    cx: &LateContext<'tcx>,
    start: &Expr<'_>,
    end: &Expr<'_>,
    limits: ast::RangeLimits,
    src: &IndexExpr<'_>,
    dst: &IndexExpr<'_>,
) -> String {
    fn print_offset(offset: MinifyingSugg<'static>) -> MinifyingSugg<'static> {
        if offset.to_string() == "0" {
            sugg::EMPTY.into()
        } else {
            offset
        }
    }

    let print_limit = |end: &Expr<'_>, end_str: &str, base: &Expr<'_>, sugg: MinifyingSugg<'static>| {
        if_chain! {
            if let ExprKind::MethodCall(method, _, len_args, _) = end.kind;
            if method.ident.name == sym::len;
            if len_args.len() == 1;
            if let Some(arg) = len_args.get(0);
            if path_to_local(arg) == path_to_local(base);
            then {
                if sugg.to_string() == end_str {
                    sugg::EMPTY.into()
                } else {
                    sugg
                }
            } else {
                match limits {
                    ast::RangeLimits::Closed => {
                        sugg + &sugg::ONE.into()
                    },
                    ast::RangeLimits::HalfOpen => sugg,
                }
            }
        }
    };

    let start_str = Sugg::hir(cx, start, "").into();
    let end_str: MinifyingSugg<'_> = Sugg::hir(cx, end, "").into();

    let (src_offset, src_limit) = match src.idx {
        StartKind::Range => (
            print_offset(apply_offset(
                &apply_offset(&start_str, &src.idx_offset),
                &dst.idx_offset,
            ))
            .into_sugg(),
            print_limit(
                end,
                end_str.to_string().as_str(),
                src.base,
                apply_offset(&apply_offset(&end_str, &src.idx_offset), &dst.idx_offset),
            )
            .into_sugg(),
        ),
        StartKind::Counter { initializer } => {
            let counter_start = Sugg::hir(cx, initializer, "").into();
            (
                print_offset(apply_offset(
                    &apply_offset(&counter_start, &src.idx_offset),
                    &dst.idx_offset,
                ))
                .into_sugg(),
                print_limit(
                    end,
                    end_str.to_string().as_str(),
                    src.base,
                    apply_offset(&apply_offset(&end_str, &src.idx_offset), &dst.idx_offset) + &counter_start
                        - &start_str,
                )
                .into_sugg(),
            )
        },
    };

    let src_base_str = snippet(cx, src.base.span, "???");

    format!(
        "{}.copy_within({}..{}, {});",
        src_base_str,
        src_offset.maybe_par(),
        src_limit.maybe_par(),
        start_str,
    )
}

fn left_is_smaller_than_right<'tcx>(
    cx: &LateContext<'tcx>,
    idx_left: &'tcx Expr<'_>,
    idx_right: &'tcx Expr<'_>,
) -> bool {
    // in order to be a memmove-type loop, read indices must be iterated
    // over at a later point than written indices. we currently enforce
    // this by ensuring that:
    //
    // - rhs is `path + offset` with offset >= 0
    // - lhs is just `path` (same path as rhs)
    if_chain! {
        if let ExprKind::Binary(idx_right_op, idx_right_lhs, idx_right_rhs) = idx_right.kind;
        if idx_right_op.node == BinOpKind::Add;
        if let Some(idx_left_local) = path_to_local(idx_left);
        if let Some(idx_right_local) = path_to_local(idx_right_lhs);
        if idx_right_local == idx_left_local;
        if let ty::Uint(_) = cx.typeck_results().expr_ty(idx_right_rhs).kind();
        then {
            true
        } else {
            false
        }
    }
}
