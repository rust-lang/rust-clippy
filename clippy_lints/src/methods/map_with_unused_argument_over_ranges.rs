use crate::methods::MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES;
use clippy_config::msrvs::{self, Msrv};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher;
use clippy_utils::source::snippet_with_applicability;
use rustc_ast::ast::RangeLimits;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{Body, Closure, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;

fn extract_count_with_applicability(
    cx: &LateContext<'_>,
    range: higher::Range<'_>,
    applicability: &mut Applicability,
) -> Option<String> {
    let start = range.start?;
    let end = range.end?;
    // TODO: This doens't handle if either the start or end are negative literals, or if the start is
    // not a literal. In the first case, we need to be careful about how we handle computing the
    // count to avoid overflows. In the second, we may need to add parenthesis to make the
    // suggestion correct.
    if let ExprKind::Lit(lit) = start.kind
        && let LitKind::Int(Pu128(lower_bound), _) = lit.node
    {
        if let ExprKind::Lit(lit) = end.kind
            && let LitKind::Int(Pu128(upper_bound), _) = lit.node
        {
            // Here we can explicitly calculate the number of iterations
            let mut count = upper_bound - lower_bound;
            if range.limits == RangeLimits::Closed {
                count += 1;
            }
            return Some(format!("{count}"));
        }
        let end_snippet = snippet_with_applicability(cx, end.span, "...", applicability).into_owned();
        if lower_bound == 0 {
            if range.limits == RangeLimits::Closed {
                return Some(format!("{end_snippet} + 1"));
            }
            return Some(end_snippet);
        }
        if range.limits == RangeLimits::Closed {
            return Some(format!("{end_snippet} - {}", lower_bound - 1));
        }
        return Some(format!("{end_snippet} - {lower_bound}"));
    }
    None
}

pub(super) fn check(cx: &LateContext<'_>, ex: &Expr<'_>, receiver: &Expr<'_>, arg: &Expr<'_>, msrv: &Msrv) {
    if !msrv.meets(msrvs::REPEAT_WITH) {
        return;
    }
    let mut applicability = Applicability::MaybeIncorrect;
    if let Some(range) = higher::Range::hir(receiver)
        && let ExprKind::Closure(Closure { body, .. }) = arg.kind
        && let Body { params: [param], .. } = cx.tcx.hir().body(*body)
        // TODO: We should also look for a named, but unused parameter here
        && matches!(param.pat.kind, PatKind::Wild)
        && let Some(count) = extract_count_with_applicability(cx, range, &mut applicability)
    {
        // TODO: Check if we can switch_to_eager_eval here and do away with `repeat_with` and instad use
        // `repeat`
        let snippet =
            snippet_with_applicability(cx, arg.span, "|| { ... }", &mut applicability).replacen("|_|", "||", 1);
        span_lint_and_sugg(
            cx,
            MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES,
            ex.span,
            "map of a closure that does not depend on its parameter over a range",
            "use",
            format!("std::iter::repeat_with({snippet}).take({count})"),
            applicability,
        );
    }
}
