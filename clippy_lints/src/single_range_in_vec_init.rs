use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::higher::{Range, VecArgs};
use clippy_utils::macros::root_macro_call_first_node;
use clippy_utils::source::{SpanRangeExt, snippet_with_context};
use clippy_utils::ty::implements_trait;
use clippy_utils::{is_no_std_crate, sym};
use rustc_ast::{LitIntType, LitKind, RangeLimits, UintTy};
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::DesugaringKind;
use std::fmt::{self, Display, Formatter};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for `Vec` or array initializations that contain only one range.
    ///
    /// ### Why is this bad?
    /// This is almost always incorrect, as it will result in a `Vec` that has only one element.
    /// Almost always, the programmer intended for it to include all elements in the range or for
    /// the end of the range to be the length instead.
    ///
    /// ### Example
    /// ```no_run
    /// let x = [0..200];
    /// ```
    /// Use instead:
    /// ```no_run
    /// // If it was intended to include every element in the range...
    /// let x = (0..200).collect::<Vec<i32>>();
    /// // ...Or if 200 was meant to be the len
    /// let x = [0; 200];
    /// ```
    #[clippy::version = "1.72.0"]
    pub SINGLE_RANGE_IN_VEC_INIT,
    suspicious,
    "checks for initialization of `Vec` or arrays which consist of a single range"
}
declare_lint_pass!(SingleRangeInVecInit => [SINGLE_RANGE_IN_VEC_INIT]);

enum SuggestedType {
    Vec,
    Array,
}

impl SuggestedType {
    fn starts_with(&self) -> &'static str {
        if matches!(self, SuggestedType::Vec) {
            "vec!"
        } else {
            "["
        }
    }

    fn ends_with(&self) -> &'static str {
        if matches!(self, SuggestedType::Vec) { "" } else { "]" }
    }
}

impl Display for SuggestedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if matches!(&self, SuggestedType::Vec) {
            write!(f, "a `Vec`")
        } else {
            write!(f, "an array")
        }
    }
}

impl LateLintPass<'_> for SingleRangeInVecInit {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        // inner_expr: `vec![0..200]` or `[0..200]`
        //                   ^^^^^^       ^^^^^^^
        // span: `vec![0..200]` or `[0..200]`
        //        ^^^^^^^^^^^^      ^^^^^^^^
        // suggested_type: What to print, "an array" or "a `Vec`"
        let (inner_expr, span, suggested_type) = if let ExprKind::Array([inner_expr]) = expr.kind
            && !expr.span.from_expansion()
        {
            (inner_expr, expr.span, SuggestedType::Array)
        } else if let Some(macro_call) = root_macro_call_first_node(cx, expr)
            && let Some(VecArgs::Vec([expr])) = VecArgs::hir(cx, expr)
        {
            (expr, macro_call.span, SuggestedType::Vec)
        } else {
            return;
        };

        // Use higher::Range to support all range types: a..b, ..b, a.., a..=b
        let Some(range) = Range::hir(cx, inner_expr) else {
            return;
        };

        if !inner_expr.span.is_desugaring(DesugaringKind::RangeExpr) {
            return;
        }

        // Get type from whichever part exists
        let ty = range.start.or(range.end).map_or_else(
            || cx.typeck_results().expr_ty(inner_expr),
            |e| cx.typeck_results().expr_ty(e),
        );

        if let Some(snippet) = span.get_source_text(cx)
            // `is_from_proc_macro` will skip any `vec![]`. Let's not!
            && snippet.starts_with(suggested_type.starts_with())
            && snippet.ends_with(suggested_type.ends_with())
        {
            let mut applicability = Applicability::MaybeIncorrect;

            // Generate snippets for start and end
            let start_snippet = range
                .start
                .map(|e| snippet_with_context(cx, e.span, span.ctxt(), "..", &mut applicability).0);
            let end_snippet = range
                .end
                .map(|e| snippet_with_context(cx, e.span, span.ctxt(), "..", &mut applicability).0);

            // Determine if we can suggest collect (for ranges that implement RangeStepIterator)
            let should_emit_every_value = if let Some(step_def_id) = cx.tcx.get_diagnostic_item(sym::range_step)
                && implements_trait(cx, ty, step_def_id, &[])
            {
                true
            } else {
                false
            };

            // Determine if we can suggest [start; len] or vec![start; len]
            // For this, we need an end value that is a usize literal
            let should_emit_of_len = if let Some(end_expr) = range.end
                && let Some(copy_def_id) = cx.tcx.lang_items().copy_trait()
                && implements_trait(cx, ty, copy_def_id, &[])
                && let ExprKind::Lit(lit_kind) = end_expr.kind
                && let LitKind::Int(.., suffix_type) = lit_kind.node
                && let LitIntType::Unsigned(UintTy::Usize) | LitIntType::Unsuffixed = suffix_type
            {
                true
            } else {
                false
            };

            if should_emit_every_value || should_emit_of_len {
                span_lint_and_then(
                    cx,
                    SINGLE_RANGE_IN_VEC_INIT,
                    span,
                    format!("{suggested_type} of `Range` that is only one element"),
                    |diag| {
                        // Build the collect suggestion text
                        // Note: RangeTo (..end) and RangeFrom (start..) need special handling:
                        // - ..end is not an iterator, so we use (0..end)
                        // - start.. is infinite, so we don't suggest collect for it
                        let collect_range_text = match (start_snippet.as_deref(), end_snippet.as_deref(), range.limits)
                        {
                            (Some(start), Some(end), RangeLimits::HalfOpen) => format!("{start}..{end}"),
                            (Some(start), Some(end), RangeLimits::Closed) => format!("{start}..={end}"),
                            (None, Some(end), _) => format!("0..{end}"), // ..end becomes 0..end
                            // start.. is infinite, skip collect suggestion for it
                            _ => return,
                        };

                        // Suggest using .collect() for the entire range
                        if should_emit_every_value && !is_no_std_crate(cx) {
                            diag.span_suggestion(
                                span,
                                "if you wanted a `Vec` that contains the entire range, try",
                                format!("({collect_range_text}).collect::<std::vec::Vec<{ty}>>()"),
                                applicability,
                            );
                        }

                        // Suggest using array/vec initialization for length
                        if should_emit_of_len {
                            // For ranges like `..end`, use 0 as default start
                            // For ranges like `start..end`, use the start value
                            let start_value = start_snippet.as_deref().unwrap_or("0");

                            // For `..end`, end is the length
                            // For `start..end`, end is the length (and start is the value to repeat)
                            let len_value = end_snippet.as_deref().expect("should have end value");

                            diag.span_suggestion(
                                inner_expr.span,
                                format!("if you wanted {suggested_type} of len {len_value}, try"),
                                format!("{start_value}; {len_value}"),
                                applicability,
                            );
                        }
                    },
                );
            }
        }
    }
}
