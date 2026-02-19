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
use rustc_middle::ty::Ty;
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use std::borrow::Cow;
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
    ///
    /// This lint also triggers on inclusive and open-ended ranges:
    /// ```no_run
    /// let x = [0..=200]; // inclusive range
    /// let x = [..200];   // no start
    /// let x = [..=200];  // no start, inclusive
    /// ```
    ///
    /// ### Notes
    /// - Infinite ranges (e.g. `a..`, `..`) are ignored.
    /// - Floating-point ranges are ignored because `Step` is not implemented.
    /// - For ranges without a start (`..N` or `..=N`), only `.collect::<Vec<_>>()` is suggested.
    /// - Array-of-len suggestion is only made when the end type is `usize`.
    /// - Both inclusive (`..=`) and exclusive (`..`) ranges are detected.
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
        if matches!(self, SuggestedType::Vec) {
            write!(f, "a `Vec`")
        } else {
            write!(f, "an array")
        }
    }
}

impl LateLintPass<'_> for SingleRangeInVecInit {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some((inner_expr, span, suggested_type)) = into_parts(cx, expr) else {
            return;
        };

        let Some(range) = Range::hir(cx, inner_expr) else {
            return;
        };

        let (start, Some(end)) = (range.start, range.end) else {
            return;
        };

        if !valid_syntax(cx, span, &suggested_type) {
            return;
        }

        let mut applicability = Applicability::MaybeIncorrect;
        let ty = start.map_or(cx.typeck_results().expr_ty(end), |start| {
            cx.typeck_results().expr_ty(start)
        });

        let should_emit_every_value = start.is_none_or(|_| implements_range_step(cx, ty));
        let should_emit_of_len = is_copy_and_usize(cx, end, ty) && start.is_some();
        if !should_emit_every_value && !should_emit_of_len {
            return;
        }

        let start_snippet = start.map_or(Cow::Borrowed("0"), |start| {
            snippet_with_context(cx, start.span, span.ctxt(), "..", &mut applicability).0
        });

        let end_snippet = snippet_with_context(cx, end.span, span.ctxt(), "..", &mut applicability).0;
        let range_limits = extract_range_limits(range.limits);
        span_lint_and_then(
            cx,
            SINGLE_RANGE_IN_VEC_INIT,
            span,
            format!("{suggested_type} of `Range` that is only one element"),
            |diag| {
                if should_emit_every_value && !is_no_std_crate(cx) {
                    diag.span_suggestion(
                        span,
                        "if you wanted a `Vec` that contains the entire range, try",
                        format!("({start_snippet}{range_limits}{end_snippet}).collect::<std::vec::Vec<{ty}>>()"),
                        applicability,
                    );
                }

                if should_emit_of_len {
                    diag.span_suggestion(
                        inner_expr.span,
                        format!("if you wanted {suggested_type} of len {end_snippet}, try"),
                        format!("{start_snippet}; {end_snippet}"),
                        applicability,
                    );
                }
            },
        );
    }
}

// Extracts the "core" expression inside a single-element array or vec![]
// inner_expr: the range expression inside (e.g., 0..200, ..200, 0..=200)
// span: the entire array or vec![] expression
// suggested_type: either "array" or "Vec" to display in lint message
fn into_parts<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, Span, SuggestedType)> {
    if let ExprKind::Array([inner_expr]) = expr.kind
        && !expr.span.from_expansion()
    {
        Some((inner_expr, expr.span, SuggestedType::Array))
    } else if let Some(macro_call) = root_macro_call_first_node(cx, expr)
        && let Some(VecArgs::Vec([expr])) = VecArgs::hir(cx, expr)
    {
        Some((expr, macro_call.span, SuggestedType::Vec))
    } else {
        None
    }
}

fn extract_range_limits(limits: RangeLimits) -> &'static str {
    match limits {
        RangeLimits::HalfOpen => "..",
        RangeLimits::Closed => "..=",
    }
}

fn implements_range_step<'a>(cx: &LateContext<'a>, ty: Ty<'a>) -> bool {
    cx.tcx
        .get_diagnostic_item(sym::range_step)
        .is_some_and(|step_def_id| implements_trait(cx, ty, step_def_id, &[]))
}

fn valid_syntax(cx: &LateContext<'_>, span: Span, suggested_type: &SuggestedType) -> bool {
    let Some(snippet) = span.get_source_text(cx) else {
        return false;
    };
    snippet.starts_with(suggested_type.starts_with()) && snippet.ends_with(suggested_type.ends_with())
}

fn is_copy_and_usize<'a>(cx: &LateContext<'a>, end: &Expr<'a>, ty: Ty<'a>) -> bool {
    if let Some(copy_def_id) = cx.tcx.lang_items().copy_trait()
        && implements_trait(cx, ty, copy_def_id, &[])
        && let ExprKind::Lit(lit_kind) = end.kind
        && let LitKind::Int(.., suffix_type) = lit_kind.node
        && let LitIntType::Unsigned(UintTy::Usize) | LitIntType::Unsuffixed = suffix_type
    {
        true
    } else {
        false
    }
}
