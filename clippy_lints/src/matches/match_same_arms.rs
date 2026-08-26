use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::res::MaybeResPath as _;
use clippy_utils::source::SpanExt as _;
use clippy_utils::{SpanlessEq, fulfill_or_allowed, hash_expr, is_lint_allowed, search_same};
use core::iter;
use itertools::Itertools as _;
use rustc_arena::DroplessArena;
use rustc_errors::Applicability;
use rustc_hir::{Arm, Expr, HirId, HirIdMap, HirIdMapEntry, HirIdSet, Pat, PatKind};
use rustc_lint::builtin::NON_EXHAUSTIVE_OMITTED_PATTERNS;
use rustc_lint::{LateContext, LintContext as _};
use rustc_middle::ty::TypeckResults;
use rustc_span::{Span, SyntaxContext};

use super::MATCH_SAME_ARMS;
use super::pat_overlap::NormalizedPat;

#[expect(clippy::too_many_lines)]
pub(super) fn check<'tcx>(cx: &LateContext<'tcx>, arms: &'tcx [Arm<'_>]) {
    let hash = |&(_, arm): &(_, &Arm<'_>)| hash_expr(cx, arm.body);

    let arena = DroplessArena::default();
    let normalized_pats: Vec<_> = arms
        .iter()
        .map(|a| NormalizedPat::from_pat(cx, &arena, a.pat))
        .collect();

    // The furthest forwards a pattern can move without semantic changes
    let forwards_blocking_idxs: Vec<_> = normalized_pats
        .iter()
        .enumerate()
        .map(|(i, pat)| {
            (normalized_pats[i + 1..].iter().enumerate())
                .find_map(|(j, other)| pat.has_overlapping_values(other).then_some(i + 1 + j))
                .unwrap_or(normalized_pats.len())
        })
        .collect();

    // The furthest backwards a pattern can move without semantic changes
    let backwards_blocking_idxs: Vec<_> = normalized_pats
        .iter()
        .enumerate()
        .map(|(i, pat)| {
            iter::zip(
                normalized_pats[..i].iter().enumerate().rev(),
                forwards_blocking_idxs[..i].iter().copied().rev(),
            )
            .skip_while(|&(_, forward_block)| forward_block > i)
            .find_map(|((j, other), forward_block)| {
                (forward_block == i || pat.has_overlapping_values(other)).then_some(j)
            })
            .unwrap_or(0)
        })
        .collect();

    let eq = |&(lindex, lhs): &(usize, &Arm<'_>), &(rindex, rhs): &(usize, &Arm<'_>)| -> bool {
        let min_index = usize::min(lindex, rindex);
        let max_index = usize::max(lindex, rindex);

        let check_eq_with_pat = |expr_a: &Expr<'_>, expr_b: &Expr<'_>| {
            let mut local_map: HirIdMap<HirId> = HirIdMap::default();
            let eq_fallback = |a_typeck_results: &TypeckResults<'tcx>,
                               a: &Expr<'_>,
                               b_typeck_results: &TypeckResults<'tcx>,
                               b: &Expr<'_>| {
                if let Some(a_id) = a.res_local_id()
                    && let Some(b_id) = b.res_local_id()
                    && let entry = match local_map.entry(a_id) {
                        HirIdMapEntry::Vacant(entry) => entry,
                        // check if using the same bindings as before
                        HirIdMapEntry::Occupied(entry) => return *entry.get() == b_id,
                    }
                    // the names technically don't have to match; this makes the lint more conservative
                    && cx.tcx.hir_name(a_id) == cx.tcx.hir_name(b_id)
                    && a_typeck_results.expr_ty(a) == b_typeck_results.expr_ty(b)
                    && pat_contains_local(lhs.pat, a_id)
                    && pat_contains_local(rhs.pat, b_id)
                {
                    entry.insert(b_id);
                    true
                } else {
                    false
                }
            };

            SpanlessEq::new(cx)
                .expr_fallback(eq_fallback)
                .eq_expr(SyntaxContext::root(), expr_a, expr_b)
                // these checks could be removed to allow unused bindings
                && bindings_eq(lhs.pat, local_map.keys().copied().collect())
                && bindings_eq(rhs.pat, local_map.values().copied().collect())
        };

        let check_same_guard = || match (&lhs.guard, &rhs.guard) {
            (None, None) => true,
            (Some(lhs_guard), Some(rhs_guard)) => check_eq_with_pat(lhs_guard, rhs_guard),
            _ => false,
        };

        let check_same_body = || check_eq_with_pat(lhs.body, rhs.body);

        // Arms with different guard are ignored, those can’t always be merged together
        // If both arms overlap with an arm in between then these can't be merged either.
        !(backwards_blocking_idxs[max_index] > min_index && forwards_blocking_idxs[min_index] < max_index)
            && check_same_guard()
            && check_same_body()
    };

    let indexed_arms: Vec<(usize, &Arm<'_>)> = arms.iter().enumerate().collect();
    for mut group in search_same(&indexed_arms, hash, eq) {
        // Filter out (and fulfill) `#[allow]`ed and `#[expect]`ed arms
        group.retain(|(_, arm)| !fulfill_or_allowed(cx, MATCH_SAME_ARMS, [arm.hir_id]));

        if group.len() < 2 {
            continue;
        }

        span_lint_and_then(
            cx,
            MATCH_SAME_ARMS,
            group.iter().map(|(_, arm)| arm.span).collect_vec(),
            "these match arms have identical bodies",
            |diag| {
                diag.help("if this is unintentional make the arms return different values");

                if let [prev @ .., (_, last)] = group.as_slice()
                    && is_wildcard_arm(last.pat)
                    && is_lint_allowed(cx, NON_EXHAUSTIVE_OMITTED_PATTERNS, last.hir_id)
                {
                    diag.span_label(last.span, "the wildcard arm");

                    let s = if prev.len() > 1 { "s" } else { "" };
                    diag.multipart_suggestion(
                        format!("otherwise remove the non-wildcard arm{s}"),
                        prev.iter()
                            .map(|(_, arm)| (adjusted_arm_span(cx, arm.span), String::new()))
                            .collect(),
                        Applicability::MaybeIncorrect,
                    );
                } else if let &[&(first_idx, _), .., &(last_idx, _)] = group.as_slice() {
                    let back_block = backwards_blocking_idxs[last_idx];
                    let split = if back_block < first_idx
                        || (back_block == 0 && forwards_blocking_idxs[first_idx] <= last_idx)
                    {
                        group.split_first()
                    } else {
                        group.split_last()
                    };

                    if let Some(((_, dest), src)) = split
                        && let Some(pat_snippets) = group
                            .iter()
                            .map(|(_, arm)| arm.pat.span.get_text(cx))
                            .collect::<Option<Vec<_>>>()
                    {
                        let suggs = src
                            .iter()
                            .map(|(_, arm)| (adjusted_arm_span(cx, arm.span), String::new()))
                            .chain([(dest.pat.span, pat_snippets.iter().join(" | "))])
                            .collect_vec();

                        diag.multipart_suggestion(
                            "otherwise merge the patterns into a single arm",
                            suggs,
                            Applicability::MaybeIncorrect,
                        );
                    }
                }
            },
        );
    }
}

/// Extend arm's span to include the comma and whitespaces after it.
fn adjusted_arm_span(cx: &LateContext<'_>, span: Span) -> Span {
    let source_map = cx.sess().source_map();
    source_map
        .span_extend_while(span, |c| c == ',' || c.is_ascii_whitespace())
        .unwrap_or(span)
}

fn pat_contains_local(pat: &Pat<'_>, id: HirId) -> bool {
    let mut result = false;
    pat.walk_short(|p| {
        result |= matches!(p.kind, PatKind::Binding(_, binding_id, ..) if binding_id == id);
        !result
    });
    result
}

/// Returns true if all the bindings in the `Pat` are in `ids` and vice versa
fn bindings_eq(pat: &Pat<'_>, mut ids: HirIdSet) -> bool {
    let mut result = true;
    // FIXME(rust/#120456) - is `swap_remove` correct?
    pat.each_binding_or_first(&mut |_, id, _, _| result &= ids.swap_remove(&id));
    result && ids.is_empty()
}

fn is_wildcard_arm(pat: &Pat<'_>) -> bool {
    match pat.kind {
        PatKind::Wild => true,
        PatKind::Or([.., last]) => matches!(last.kind, PatKind::Wild),
        _ => false,
    }
}
