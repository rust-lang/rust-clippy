use clippy_utils::diagnostics::span_lint_and_then;
use clippy_utils::is_from_proc_macro;
use clippy_utils::source::{FileRangeExt, SpanExt};
use rustc_ast::{Local, TyKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Detects when a variable is declared with an explicit type of `_`.
    /// ### Why is this bad?
    /// It adds noise, `: _` provides zero clarity or utility.
    /// ### Example
    /// ```rust,ignore
    /// let my_number: _ = 1;
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// let my_number = 1;
    /// ```
    #[clippy::version = "1.70.0"]
    pub LET_WITH_TYPE_UNDERSCORE,
    complexity,
    "unneeded underscore type (`_`) in a variable declaration"
}
declare_lint_pass!(UnderscoreTyped => [LET_WITH_TYPE_UNDERSCORE]);

impl EarlyLintPass for UnderscoreTyped {
    fn check_local(&mut self, cx: &EarlyContext<'_>, local: &Local) {
        if let Some(ty) = &local.ty // Ensure that it has a type defined
            && let TyKind::Infer = ty.kind // that type is '_'
            && local.span.eq_ctxt(ty.span)
            && let sm = cx.sess().source_map()
            && !local.span.in_external_macro(sm)
            && !is_from_proc_macro(cx, &**ty)
            && let Some(span_to_remove) = ty.span.map_range(cx, |scx, range| {
                range.with_leading_whitespace(scx)?
                    .with_leading_match(scx, ':')?
                    .with_leading_whitespace(scx)
            })
        {
            span_lint_and_then(
                cx,
                LET_WITH_TYPE_UNDERSCORE,
                local.span,
                "variable declared with type underscore",
                |diag| {
                    diag.span_suggestion_verbose(
                        span_to_remove,
                        "remove the explicit type `_` declaration",
                        "",
                        Applicability::MachineApplicable,
                    );
                },
            );
        }
    }
}
