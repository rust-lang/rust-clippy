//! Lint a `match` or `if let .. { .. } else { .. }` expr that could be replaced by `matches!`

use super::REDUNDANT_PATTERN_MATCHING;
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::has_let_expr;
use clippy_utils::source::{snippet_with_applicability, snippet_with_context};
use clippy_utils::{is_lint_allowed, is_wild, span_contains_comment};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Arm, BorrowKind, Expr, ExprKind, Pat, PatKind, QPath};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::Spanned;

use super::MATCH_LIKE_MATCHES_MACRO;

pub(crate) fn check_if_let<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    let_pat: &'tcx Pat<'_>,
    let_expr: &'tcx Expr<'_>,
    then_expr: &'tcx Expr<'_>,
    else_expr: &'tcx Expr<'_>,
) {
    if !span_contains_comment(cx, expr.span)
        && cx.typeck_results().expr_ty(expr).is_bool()
        && let Some(b0) = find_bool_lit(then_expr)
        && let Some(b1) = find_bool_lit(else_expr)
        && b0 != b1
    {
        if !is_lint_allowed(cx, REDUNDANT_PATTERN_MATCHING, let_pat.hir_id) && is_some_wild(let_pat.kind) {
            return;
        }

        // The suggestion may be incorrect, because some arms can have `cfg` attributes
        // evaluated into `false` and so such arms will be stripped before.
        let mut applicability = Applicability::MaybeIncorrect;
        let pat = snippet_with_applicability(cx, let_pat.span, "..", &mut applicability);

        // strip potential borrows (#6503), but only if the type is a reference
        let mut ex_new = let_expr;
        if let ExprKind::AddrOf(BorrowKind::Ref, .., ex_inner) = let_expr.kind
            && let ty::Ref(..) = cx.typeck_results().expr_ty(ex_inner).kind()
        {
            ex_new = ex_inner;
        }

        let (snippet, _) = snippet_with_context(cx, ex_new.span, expr.span.ctxt(), "..", &mut applicability);
        span_lint_and_then(
            cx,
            MATCH_LIKE_MATCHES_MACRO,
            expr.span,
            "`if let .. else` expression looks like `matches!` macro",
            |diag| {
                diag.span_suggestion_verbose(
                    expr.span,
                    "use `matches!` directly",
                    format!("{}matches!({snippet}, {pat})", if b0 { "" } else { "!" }),
                    applicability,
                );
            },
        );
    }
}

pub(super) fn check_match<'tcx>(
    cx: &LateContext<'tcx>,
    e: &'tcx Expr<'_>,
    scrutinee: &'tcx Expr<'_>,
    arms: &'tcx [Arm<'tcx>],
) -> bool {
    if arms.len() < 2 || span_contains_comment(cx, e.span) || !cx.typeck_results().expr_ty(e).is_bool() {
        return false;
    }

    let Some(arm_values) = arms
        .iter()
        .map(|arm| find_bool_lit(arm.body))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };

    let (selected_value, selected_arms, guard) = if let [first_arm, last_arm] = arms {
        // A two-arm match may preserve a non-let guard on its first arm.
        if arm_values[0] == arm_values[1] || !is_wild(last_arm.pat) || first_arm.guard.is_some_and(has_let_expr) {
            return false;
        }

        (arm_values[0], vec![first_arm], first_arm.guard)
    } else {
        // Longer matches combine guard-free arms with the selected result into an or-pattern.
        // Guards can't be combined into an or-pattern. Attributes may also remove an arm before
        // linting, which could change which result should be represented by the pattern.
        if arms
            .iter()
            .any(|arm| !cx.tcx.hir_attrs(arm.hir_id).is_empty() || arm.guard.is_some())
        {
            return false;
        }

        // A wildcard can't be part of the suggested pattern, so represent the opposite result. If
        // there isn't one, represent the `true` arms directly.
        let selected_value = arms
            .iter()
            .position(|arm| is_wild(arm.pat))
            .is_none_or(|index| !arm_values[index]);

        if !arm_values.contains(&selected_value) || !arm_values.contains(&!selected_value) {
            return false;
        }

        let selected_arms: Vec<_> = arms
            .iter()
            .zip(&arm_values)
            .filter(|&(_, &value)| value == selected_value)
            .map(|(arm, _)| arm)
            .collect();

        // An earlier arm returning the opposite value takes precedence over a later selected arm.
        // Moving that selected pattern into `matches!` is only sound when the two cannot overlap.
        for (index, (arm, &value)) in arms.iter().zip(&arm_values).enumerate() {
            if value == selected_value
                && arms[..index]
                    .iter()
                    .zip(&arm_values[..index])
                    .any(|(earlier_arm, &earlier_value)| {
                        earlier_value != selected_value
                            && super::pat_overlap::patterns_overlap(cx, earlier_arm.pat, arm.pat)
                    })
            {
                return false;
            }
        }

        (selected_value, selected_arms, None)
    };

    for arm in &selected_arms {
        let pat = arm.pat;
        if !is_lint_allowed(cx, REDUNDANT_PATTERN_MATCHING, pat.hir_id) && is_some_wild(pat.kind) {
            return false;
        }
    }

    // The suggestion may be incorrect, because some arms can have `cfg` attributes evaluated into
    // `false` and so such arms will be stripped before.
    let mut applicability = Applicability::MaybeIncorrect;
    let pat = {
        use itertools::Itertools as _;
        selected_arms
            .iter()
            .map(|arm| snippet_with_applicability(cx, arm.pat.span, "..", &mut applicability))
            .join(" | ")
    };
    let pat_and_guard = if let Some(g) = guard {
        format!(
            "{pat} if {}",
            snippet_with_applicability(cx, g.span, "..", &mut applicability)
        )
    } else {
        pat
    };

    // strip potential borrows (#6503), but only if the type is a reference
    let mut ex_new = scrutinee;
    if let ExprKind::AddrOf(BorrowKind::Ref, .., ex_inner) = scrutinee.kind
        && let ty::Ref(..) = cx.typeck_results().expr_ty(ex_inner).kind()
    {
        ex_new = ex_inner;
    }

    let (snippet, _) = snippet_with_context(cx, ex_new.span, e.span.ctxt(), "..", &mut applicability);
    span_lint_and_then(
        cx,
        MATCH_LIKE_MATCHES_MACRO,
        e.span,
        "match expression looks like `matches!` macro",
        |diag| {
            diag.span_suggestion_verbose(
                e.span,
                "use `matches!` directly",
                format!(
                    "{}matches!({snippet}, {pat_and_guard})",
                    if selected_value { "" } else { "!" }
                ),
                applicability,
            );
        },
    );
    true
}

/// Extract a `bool` or `{ bool }`
fn find_bool_lit(ex: &Expr<'_>) -> Option<bool> {
    match ex.kind {
        ExprKind::Lit(Spanned {
            node: LitKind::Bool(b), ..
        }) => Some(b),
        ExprKind::Block(
            rustc_hir::Block {
                stmts: [],
                expr: Some(exp),
                ..
            },
            _,
        ) => {
            if let ExprKind::Lit(Spanned {
                node: LitKind::Bool(b), ..
            }) = exp.kind
            {
                Some(b)
            } else {
                None
            }
        },
        _ => None,
    }
}

/// Checks whether a pattern is `Some(_)`
fn is_some_wild(pat_kind: PatKind<'_>) -> bool {
    match pat_kind {
        PatKind::TupleStruct(QPath::Resolved(_, path), [first, ..], _) if is_wild(first) => {
            let name = path.segments[0].ident;
            name.name == rustc_span::sym::Some
        },
        _ => false,
    }
}
