use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::sugg::{Sugg, make_binop};
use clippy_utils::{eq_expr_value, get_parent_expr};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::LateContext;
use rustc_span::SyntaxContext;

use super::DOUBLE_COMPARISONS;

/// Returns the operator to use when combining two comparisons sharing the same operands with
/// `chain_op` (`||` or `&&`), e.g. `x == y || x < y` combines into `x <= y`.
fn merged_op(chain_op: BinOpKind, lop: BinOpKind, rop: BinOpKind) -> Option<BinOpKind> {
    Some(match (chain_op, lop, rop) {
        // x == y || x < y => x <= y
        (BinOpKind::Or, BinOpKind::Eq, BinOpKind::Lt) | (BinOpKind::Or, BinOpKind::Lt, BinOpKind::Eq) => BinOpKind::Le,
        // x == y || x > y => x >= y
        (BinOpKind::Or, BinOpKind::Eq, BinOpKind::Gt) | (BinOpKind::Or, BinOpKind::Gt, BinOpKind::Eq) => BinOpKind::Ge,
        // x < y || x > y => x != y
        (BinOpKind::Or, BinOpKind::Lt, BinOpKind::Gt) | (BinOpKind::Or, BinOpKind::Gt, BinOpKind::Lt) => BinOpKind::Ne,
        // x <= y && x >= y => x == y
        (BinOpKind::And, BinOpKind::Le, BinOpKind::Ge) | (BinOpKind::And, BinOpKind::Ge, BinOpKind::Le) => {
            BinOpKind::Eq
        },
        // x != y && x >= y => x > y
        (BinOpKind::And, BinOpKind::Ne, BinOpKind::Ge) | (BinOpKind::And, BinOpKind::Ge, BinOpKind::Ne) => {
            BinOpKind::Gt
        },
        // x != y && x <= y => x < y
        (BinOpKind::And, BinOpKind::Ne, BinOpKind::Le) | (BinOpKind::And, BinOpKind::Le, BinOpKind::Ne) => {
            BinOpKind::Lt
        },
        _ => return None,
    })
}

/// Collects the terms of a chain of `op` (`a || b || c` gives `[a, b, c]`). Anything that is not
/// part of the chain, like a call or a group using the other operator, is a single term.
fn flatten<'hir>(op: BinOpKind, expr: &'hir Expr<'hir>, terms: &mut Vec<&'hir Expr<'hir>>) {
    if let ExprKind::Binary(bin_op, lhs, rhs) = expr.kind
        && bin_op.node == op
        && !expr.span.from_expansion()
    {
        flatten(op, lhs, terms);
        flatten(op, rhs, terms);
    } else {
        terms.push(expr);
    }
}

/// Returns the operator and operands of `term` if it is a binary expression written by the user.
fn as_binary<'hir>(term: &'hir Expr<'hir>) -> Option<(BinOpKind, &'hir Expr<'hir>, &'hir Expr<'hir>)> {
    if let ExprKind::Binary(op, lhs, rhs) = term.kind
        && !term.span.from_expansion()
    {
        Some((op.node, lhs, rhs))
    } else {
        None
    }
}

/// Returns whether `expr` is a comparison or a chain of `op`, so it might hold a comparison to merge.
fn may_contain_comparison(op: BinOpKind, expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Binary(bin_op, ..) if bin_op.node == op || bin_op.node.is_comparison())
}

/// Finds the first two terms that compare the same operands and can be merged into one
/// comparison, returning their indices and the merged operator.
fn find_mergeable_pair(
    cx: &LateContext<'_>,
    ctxt: SyntaxContext,
    op: BinOpKind,
    terms: &[&Expr<'_>],
) -> Option<(usize, usize, BinOpKind)> {
    for (i, &first) in terms.iter().enumerate() {
        let Some((first_op, first_lhs, first_rhs)) = as_binary(first) else {
            continue;
        };
        for (j, &second) in terms.iter().enumerate().skip(i + 1) {
            if let Some((second_op, second_lhs, second_rhs)) = as_binary(second)
                && let Some(new_op) = merged_op(op, first_op, second_op)
                && eq_expr_value(cx, ctxt, first_lhs, second_lhs)
                && eq_expr_value(cx, ctxt, first_rhs, second_rhs)
            {
                return Some((i, j, new_op));
            }
        }
    }
    None
}

pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'_>,
    op: BinOpKind,
    lhs: &'tcx Expr<'_>,
    rhs: &'tcx Expr<'_>,
) {
    if !op.is_lazy() || expr.span.from_expansion() {
        return;
    }

    // Skip early when neither side is a comparison or a chain, as this runs on every `&&` and `||`
    if !may_contain_comparison(op, lhs) && !may_contain_comparison(op, rhs) {
        return;
    }

    // Only handle a chain once, from its outermost node. A parent from a macro expansion never
    // gets here, so it can't take over the chain.
    if let Some(parent) = get_parent_expr(cx, expr)
        && let ExprKind::Binary(parent_op, ..) = parent.kind
        && parent_op.node == op
        && !parent.span.from_expansion()
    {
        return;
    }

    let mut terms = Vec::new();
    flatten(op, lhs, &mut terms);
    flatten(op, rhs, &mut terms);

    let ctxt = expr.span.ctxt();
    let Some((i, j, new_op)) = find_mergeable_pair(cx, ctxt, op, &terms) else {
        return;
    };
    let ExprKind::Binary(_, first_lhs, first_rhs) = terms[i].kind else {
        return;
    };

    // The merged comparison takes the place of the first one, so a term in between with side
    // effects (like a call) may no longer be evaluated
    let mut applicability = if terms[i + 1..j].iter().any(|term| term.can_have_side_effects()) {
        Applicability::MaybeIncorrect
    } else {
        Applicability::MachineApplicable
    };

    let first_lhs = Sugg::hir_with_context(cx, first_lhs, ctxt, "..", &mut applicability);
    let first_rhs = Sugg::hir_with_context(cx, first_rhs, ctxt, "..", &mut applicability);
    let merged = make_binop(new_op, &first_lhs, &first_rhs);

    let mut sugg = terms
        .iter()
        .enumerate()
        .filter(|&(k, _)| k != j)
        .map(|(k, term)| {
            if k == i {
                merged.clone()
            } else {
                Sugg::hir_with_context(cx, term, ctxt, "..", &mut applicability)
            }
        })
        .reduce(|acc, term| make_binop(op, &acc, &term))
        .unwrap_or(merged);
    // A unary operator like `!` binds tighter than the comparisons and chains handled here, so its
    // operand always needs parens. Otherwise, a chain of more than two terms needs them only when
    // `expr.span` shows it was already grouped, to keep binding the same way it did before.
    let needs_paren = matches!(
        get_parent_expr(cx, expr).map(|parent| parent.kind),
        Some(ExprKind::Unary(..))
    ) || (terms.len() > 2 && expr.span != lhs.span.source_callsite().to(rhs.span.source_callsite()));
    if needs_paren {
        sugg = sugg.maybe_paren();
    }

    span_lint_and_sugg(
        cx,
        DOUBLE_COMPARISONS,
        expr.span,
        "this binary expression can be simplified",
        "try",
        sugg.to_string(),
        applicability,
    );
}
