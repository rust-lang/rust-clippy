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

        let Some(range) = Range::hir(cx, inner_expr) else {
            return;
        };

        let Some(snippet) = span.get_source_text(cx) else {
            return;
        };

        // `is_from_proc_macro` will skip any `vec![]`. Let's not!
        if !snippet.starts_with(suggested_type.starts_with()) || !snippet.ends_with(suggested_type.ends_with()) {
            return;
        }

        // Get the type from whichever bound is available
        let ty = if let Some(start) = range.start {
            cx.typeck_results().expr_ty(start)
        } else if let Some(end) = range.end {
            cx.typeck_results().expr_ty(end)
        } else {
            // RangeFull `..` - no type to infer, skip
            return;
        };

        let mut applicability = Applicability::MaybeIncorrect;

        let start_snippet = range.start.map(|start| {
            snippet_with_context(cx, start.span, span.ctxt(), "..", &mut applicability).0
        });
        let end_snippet = range.end.map(|end| {
            snippet_with_context(cx, end.span, span.ctxt(), "..", &mut applicability).0
        });

        let should_emit_every_value = if let Some(step_def_id) = cx.tcx.get_diagnostic_item(sym::range_step)
            && implements_trait(cx, ty, step_def_id, &[])
        {
            true
        } else {
            false
        };

        // Only emit the "of len" suggestion for ranges with both start and end (a..b or a..=b)
        // and when the end is an integer literal
        let should_emit_of_len = if let Some(copy_def_id) = cx.tcx.lang_items().copy_trait()
            && implements_trait(cx, ty, copy_def_id, &[])
            && range.start.is_some()
            && let Some(end_expr) = range.end
            && let ExprKind::Lit(lit_kind) = end_expr.kind
            && let LitKind::Int(.., suffix_type) = lit_kind.node
            && let LitIntType::Unsigned(UintTy::Usize) | LitIntType::Unsuffixed = suffix_type
            // Don't suggest `[start; end]` for inclusive ranges since it's confusing
            && range.limits == RangeLimits::HalfOpen
        {
            true
        } else {
            false
        };

        if should_emit_every_value || should_emit_of_len {
            // Format the range type name for the error message
            let range_type = match (range.start.is_some(), range.end.is_some(), range.limits) {
                (true, true, RangeLimits::HalfOpen) => "Range",
                (true, true, RangeLimits::Closed) => "RangeInclusive",
                (true, false, _) => "RangeFrom",
                (false, true, RangeLimits::HalfOpen) => "RangeTo",
                (false, true, RangeLimits::Closed) => "RangeToInclusive",
                (false, false, _) => "RangeFull",
            };

            span_lint_and_then(
                cx,
                SINGLE_RANGE_IN_VEC_INIT,
                span,
                format!("{suggested_type} of `{range_type}` that is only one element"),
                |diag| {
                    if should_emit_every_value && !is_no_std_crate(cx) {
                        // Format the range syntax for the suggestion
                        let range_op = if range.limits == RangeLimits::Closed {
                            "..="
                        } else {
                            ".."
                        };
                        let range_str = match (&start_snippet, &end_snippet) {
                            (Some(start), Some(end)) => format!("{start}{range_op}{end}"),
                            (Some(start), None) => format!("{start}.."),
                            (None, Some(end)) => format!("{range_op}{end}"),
                            (None, None) => "..".to_string(),
                        };

                        diag.span_suggestion(
                            span,
                            "if you wanted a `Vec` that contains the entire range, try",
                            format!("({range_str}).collect::<std::vec::Vec<{ty}>>()"),
                            applicability,
                        );
                    }

                    if should_emit_of_len {
                        if let (Some(start), Some(end)) = (&start_snippet, &end_snippet) {
                            diag.span_suggestion(
                                inner_expr.span,
                                format!("if you wanted {suggested_type} of len {end}, try"),
                                format!("{start}; {end}"),
                                applicability,
                            );
                        }
                    }
                },
            );
        }
    }
}
