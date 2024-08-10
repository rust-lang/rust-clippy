use crate::methods::MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES;
use clippy_config::msrvs::{self, Msrv};
use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::usage;
use rustc_ast::ast::RangeLimits;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{Body, Closure, Expr, ExprKind, PatKind};
use rustc_lint::LateContext;
use rustc_span::Span;

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
            let count = if upper_bound > lower_bound {
                match range.limits {
                    RangeLimits::HalfOpen => upper_bound - lower_bound,
                    RangeLimits::Closed => upper_bound - lower_bound + 1,
                }
            } else if upper_bound == lower_bound && range.limits == RangeLimits::Closed {
                1
            } else {
                0
            };
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

pub(super) fn check(
    cx: &LateContext<'_>,
    ex: &Expr<'_>,
    receiver: &Expr<'_>,
    arg: &Expr<'_>,
    msrv: &Msrv,
    method_call_span: Span,
) {
    if !msrv.meets(msrvs::REPEAT_WITH) {
        return;
    }
    let mut applicability = Applicability::MaybeIncorrect;
    if let Some(range) = higher::Range::hir(receiver)
        && let ExprKind::Closure(Closure { body, .. }) = arg.kind
        && let body_hir = cx.tcx.hir().body(*body)
        && let Body { params: [param], .. } = body_hir
        && !usage::BindingUsageFinder::are_params_used(cx, body_hir)
        && let Some(count) = extract_count_with_applicability(cx, range, &mut applicability)
    {
        // TODO: Check if we can switch_to_eager_eval here and do away with `repeat_with` and instad use
        // `repeat`
        span_lint_and_then(
            cx,
            MAP_WITH_UNUSED_ARGUMENT_OVER_RANGES,
            ex.span,
            "map of a closure that does not depend on its parameter over a range",
            |diag| {
                diag.multipart_suggestion(
                    "remove the explicit range and use `repeat_with` and `take`",
                    vec![
                        (receiver.span.to(method_call_span), "std::iter::repeat_with".to_owned()),
                        (param.span, String::new()),
                        (ex.span.shrink_to_hi(), format!(".take({count})")),
                    ],
                    applicability,
                );
            },
        );
    }
}
