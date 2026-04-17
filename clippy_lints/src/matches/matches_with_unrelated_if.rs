use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::source::snippet;
use clippy_utils::{is_direct_expn_of, sym};
use rustc_errors::Applicability;
use rustc_hir::{Arm, Expr, PatKind};
use rustc_lint::LateContext;

use super::MATCHES_WITH_UNRELATED_IF;

/// Checks for `matches!(expr, Pattern if guard)` where `guard` does not use any
/// variable bound by `Pattern`.
pub(super) fn check<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    ex: &'tcx Expr<'tcx>,
    arms: &'tcx [Arm<'tcx>],
) {
    // Only fire inside a `matches!` macro call
    if is_direct_expn_of(expr.span, sym::matches).is_none() {
        return;
    }

    // `matches!` expands to exactly 2 arms: the user's arm and a wildcard `_ => false`
    let [arm, _] = arms else {
        return;
    };

    // Only relevant when the arm has a guard
    let Some(guard) = arm.guard else {
        return;
    };

    // Collect all HirIds of bindings introduced by the pattern
    let mut binding_ids = Vec::new();
    arm.pat.walk(|pat| {
        if let PatKind::Binding(_, hir_id, _, _) = pat.kind {
            binding_ids.push(hir_id);
        }
        true
    });

    // Check whether the guard uses any of the pattern's bindings
    let guard_uses_binding = binding_ids
        .iter()
        .any(|id| clippy_utils::visitors::is_local_used(cx, guard, *id));

    if guard_uses_binding {
        return;
    }

    // The guard is unrelated to the pattern — emit the lint
    let call_site = expr.span.source_callsite();
    let scrutinee_snip = snippet(cx, ex.span, "..");
    let pat_snip = snippet(cx, arm.pat.span, "..");
    let guard_snip = snippet(cx, guard.span, "..");

    span_lint_and_sugg(
        cx,
        MATCHES_WITH_UNRELATED_IF,
        call_site,
        "the `if` guard in `matches!` does not use any variable bound by the pattern",
        "move the guard outside of `matches!`",
        format!("matches!({scrutinee_snip}, {pat_snip}) && {guard_snip}"),
        Applicability::MachineApplicable,
    );
}
