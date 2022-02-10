use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::{expr_block, snippet};
use clippy_utils::ty::{implements_trait, match_type, peel_mid_ty_refs};
use clippy_utils::{
    is_lint_allowed, is_unit_expr, is_wild, paths, peel_blocks, peel_hir_pat_refs, peel_n_hir_expr_refs,
};
use core::cmp::max;
use rustc_errors::Applicability;
use rustc_hir::{Arm, Block, Expr, ExprKind, Pat, PatField, PatKind};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty};

use super::{MATCH_BOOL, SINGLE_MATCH, SINGLE_MATCH_ELSE};

#[rustfmt::skip]
pub(crate) fn check(cx: &LateContext<'_>, ex: &Expr<'_>, arms: &[Arm<'_>], expr: &Expr<'_>) {
    if arms.len() == 2 && arms[0].guard.is_none() && arms[1].guard.is_none() {
        if expr.span.from_expansion() {
            // Don't lint match expressions present in
            // macro_rules! block
            return;
        }
        if let PatKind::Or(..) = arms[0].pat.kind {
            // don't lint for or patterns for now, this makes
            // the lint noisy in unnecessary situations
            return;
        }
        let els = arms[1].body;
        let els = if is_unit_expr(peel_blocks(els)) {
            None
        } else if let ExprKind::Block(Block { stmts, expr: block_expr, .. }, _) = els.kind {
            if stmts.len() == 1 && block_expr.is_none() || stmts.is_empty() && block_expr.is_some() {
                // single statement/expr "else" block, don't lint
                return;
            }
            // block with 2+ statements or 1 expr and 1+ statement
            Some(els)
        } else {
            // not a block, don't lint
            return;
        };

        let ty = cx.typeck_results().expr_ty(ex);
        if *ty.kind() != ty::Bool || is_lint_allowed(cx, MATCH_BOOL, ex.hir_id) {
            check_single_pattern(cx, ex, arms, expr, els);
            check_opt_like(cx, ex, arms, expr, ty, els);
        }
    }
}

fn check_single_pattern(
    cx: &LateContext<'_>,
    ex: &Expr<'_>,
    arms: &[Arm<'_>],
    expr: &Expr<'_>,
    els: Option<&Expr<'_>>,
) {
    if is_wild(arms[1].pat) {
        report_single_pattern(cx, ex, arms, expr, els);
    }
}

fn report_single_pattern(
    cx: &LateContext<'_>,
    ex: &Expr<'_>,
    arms: &[Arm<'_>],
    expr: &Expr<'_>,
    els: Option<&Expr<'_>>,
) {
    let lint = if els.is_some() { SINGLE_MATCH_ELSE } else { SINGLE_MATCH };
    let els_str = els.map_or(String::new(), |els| {
        format!(" else {}", expr_block(cx, els, None, "..", Some(expr.span)))
    });

    let (pat, pat_ref_count) = peel_hir_pat_refs(arms[0].pat);
    let (msg, sugg) = if_chain! {
        if let PatKind::Path(_) | PatKind::Lit(_) = pat.kind;
        let (ty, ty_ref_count) = peel_mid_ty_refs(cx.typeck_results().expr_ty(ex));
        if let Some(spe_trait_id) = cx.tcx.lang_items().structural_peq_trait();
        if let Some(pe_trait_id) = cx.tcx.lang_items().eq_trait();
        if ty.is_integral() || ty.is_char() || ty.is_str()
            || (implements_trait(cx, ty, spe_trait_id, &[])
                && implements_trait(cx, ty, pe_trait_id, &[ty.into()]));
        then {
            // scrutinee derives PartialEq and the pattern is a constant.
            let pat_ref_count = match pat.kind {
                // string literals are already a reference.
                PatKind::Lit(Expr { kind: ExprKind::Lit(lit), .. }) if lit.node.is_str() => pat_ref_count + 1,
                _ => pat_ref_count,
            };
            // References are only implicitly added to the pattern, so no overflow here.
            // e.g. will work: match &Some(_) { Some(_) => () }
            // will not: match Some(_) { &Some(_) => () }
            let ref_count_diff = ty_ref_count - pat_ref_count;

            // Try to remove address of expressions first.
            let (ex, removed) = peel_n_hir_expr_refs(ex, ref_count_diff);
            let ref_count_diff = ref_count_diff - removed;

            let msg = "you seem to be trying to use `match` for an equality check. Consider using `if`";
            let sugg = format!(
                "if {} == {}{} {}{}",
                snippet(cx, ex.span, ".."),
                // PartialEq for different reference counts may not exist.
                "&".repeat(ref_count_diff),
                snippet(cx, arms[0].pat.span, ".."),
                expr_block(cx, arms[0].body, None, "..", Some(expr.span)),
                els_str,
            );
            (msg, sugg)
        } else {
            let msg = "you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`";
            let sugg = format!(
                "if let {} = {} {}{}",
                snippet(cx, arms[0].pat.span, ".."),
                snippet(cx, ex.span, ".."),
                expr_block(cx, arms[0].body, None, "..", Some(expr.span)),
                els_str,
            );
            (msg, sugg)
        }
    };

    span_lint_and_sugg(
        cx,
        lint,
        expr.span,
        msg,
        "try this",
        sugg,
        Applicability::HasPlaceholders,
    );
}

fn check_opt_like<'a>(
    cx: &LateContext<'a>,
    ex: &Expr<'_>,
    arms: &[Arm<'_>],
    expr: &Expr<'_>,
    ty: Ty<'a>,
    els: Option<&Expr<'_>>,
) {
    if check_exhaustive(cx, arms[0].pat, arms[1].pat, ty) {
        report_single_pattern(cx, ex, arms, expr, els);
    }
}

/// Resturns true if the given type is one of the standard `Enum`s we know will never get any more
/// members.
fn is_known_enum(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    const CANDIDATES: &[&[&str; 3]] = &[&paths::COW, &paths::OPTION, &paths::RESULT];
    CANDIDATES.iter().any(|&ty_path| match_type(cx, ty, ty_path))
}

fn contains_only_known_enums(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    match pat.kind {
        PatKind::Path(..) => is_known_enum(cx, cx.typeck_results().pat_ty(pat)),
        PatKind::TupleStruct(..) => {
            let ty = cx.typeck_results().pat_ty(pat);
            (contains_only_wilds(pat) || contains_single_binding(pat)) && is_known_enum(cx, ty)
        },
        _ => false,
    }
}

fn peel_subpatterns<'a>(pat: &'a Pat<'a>) -> &'a Pat<'a> {
    if let PatKind::Binding(.., Some(sub)) = pat.kind {
        return peel_subpatterns(sub);
    }
    pat
}

/// Returns false if the patterns exhaustively match an enum.
fn check_exhaustive<'a>(cx: &LateContext<'a>, left: &Pat<'_>, right: &Pat<'_>, ty: Ty<'a>) -> bool {
    let left = peel_subpatterns(left);
    let right = peel_subpatterns(right);
    match (&left.kind, &right.kind) {
        (PatKind::Wild, _) | (_, PatKind::Wild) => true,
        (PatKind::Struct(_, left_fields, _), PatKind::Struct(_, right_fields, _)) => {
            check_exhaustive_structs(cx, left_fields, right_fields)
        },
        (PatKind::Tuple(left_in, left_pos), PatKind::Tuple(right_in, right_pos)) => {
            check_exhaustive_tuples(cx, left_in, left_pos, right_in, right_pos)
        },
        (PatKind::TupleStruct(_, left_in, left_pos), PatKind::TupleStruct(_, right_in, right_pos))
            if contains_only_wilds(right) && contains_only_known_enums(cx, left) =>
        {
            check_exhaustive_tuples(cx, left_in, left_pos, right_in, right_pos)
        },
        (PatKind::Binding(.., None) | PatKind::Path(_), _) if contains_only_wilds(right) => is_known_enum(cx, ty),
        _ => false,
    }
}

fn could_be_simplified(cx: &LateContext<'_>, left_pat: &Pat<'_>, right_pat: &Pat<'_>) -> bool {
    contains_only_wilds(left_pat)
        || contains_only_wilds(right_pat)
        || contains_only_known_enums(cx, left_pat) && contains_only_known_enums(cx, right_pat)
}

/// Returns true if two tuples that contain patterns `left_in` and `right_in` and `..` operators at
/// positions `left_pos` and `right_pos` form exhaustive match. This means, that two elements of
/// these tuples located at the same positions must have at least one wildcard (`_`) pattern.
fn check_exhaustive_tuples<'a>(
    cx: &LateContext<'_>,
    left_in: &'a [Pat<'a>],
    left_pos: &Option<usize>,
    right_in: &'a [Pat<'a>],
    right_pos: &Option<usize>,
) -> bool {
    // We don't actually know the position and the presence of the `..` (dotdot) operator
    // in the arms, so we need to evaluate the correct offsets here in order to iterate in
    // both arms at the same time.
    let len = max(
        left_in.len() + {
            if left_pos.is_some() { 1 } else { 0 }
        },
        right_in.len() + {
            if right_pos.is_some() { 1 } else { 0 }
        },
    );
    let mut left_pos = left_pos.unwrap_or(usize::MAX);
    let mut right_pos = right_pos.unwrap_or(usize::MAX);
    let mut left_dot_space = 0;
    let mut right_dot_space = 0;
    for i in 0..len {
        let mut found_dotdot = false;
        if i == left_pos {
            left_dot_space += 1;
            if left_dot_space < len - left_in.len() {
                left_pos += 1;
            }
            found_dotdot = true;
        }
        if i == right_pos {
            right_dot_space += 1;
            if right_dot_space < len - right_in.len() {
                right_pos += 1;
            }
            found_dotdot = true;
        }
        if found_dotdot {
            continue;
        }

        if !could_be_simplified(cx, &left_in[i - left_dot_space], &right_in[i - right_dot_space]) {
            return false;
        }
    }
    true
}

fn check_exhaustive_structs<'a>(
    cx: &LateContext<'_>,
    left_fields: &'a [PatField<'a>],
    right_fields: &'a [PatField<'a>],
) -> bool {
    left_fields
        .iter()
        .zip(right_fields.iter())
        .all(|(left_field, right_field)| could_be_simplified(cx, left_field.pat, right_field.pat))
}

/// Returns true if the given arm of pattern matching contains only wildcard patterns.
fn contains_only_wilds(pat: &Pat<'_>) -> bool {
    match pat.kind {
        PatKind::Wild => true,
        PatKind::Tuple(inner, _) | PatKind::TupleStruct(_, inner, ..) => inner.iter().all(contains_only_wilds),
        _ => false,
    }
}

fn contains_single_binding(pat: &Pat<'_>) -> bool {
    match pat.kind {
        PatKind::Binding(.., None) => true,
        PatKind::TupleStruct(_, inner, ..) => inner.iter().all(contains_single_binding),
        _ => false,
    }
}
