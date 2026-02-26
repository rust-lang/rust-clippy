use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{GenericParam, GenericParamKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Enforce that `where` bounds are used for all trait bounds.
    ///
    /// ### Why restrict this?
    /// Enforce a single style throughout a codebase.
    /// Avoid uncertainty about whether a bound should be inline
    /// or out-of-line (i.e. a where bound).
    /// Avoid complex inline bounds, which could make a function declaration more difficult to read.
    ///
    /// ### Example
    /// ```no_run
    /// fn foo<T: Clone>() {}
    /// ```
    ///
    /// Use instead:
    /// ```no_run
    /// fn foo<T>() where T: Clone {}
    /// ```
    #[clippy::version = "1.95.0"]
    pub INLINE_TRAIT_BOUNDS,
    restriction,
    "enforce that `where` bounds are used for all trait bounds"
}
declare_lint_pass!(InlineTraitBounds => [INLINE_TRAIT_BOUNDS]);

impl EarlyLintPass for InlineTraitBounds {
    fn check_generic_param(&mut self, cx: &EarlyContext<'_>, param: &GenericParam) {
        if let GenericParamKind::Type { .. } = param.kind
            && !param.bounds.is_empty()
        {
            span_lint_and_help(
                cx,
                INLINE_TRAIT_BOUNDS,
                param.span(),
                "inline trait bounds",
                None,
                "consider a `where` bound instead",
            );
        }
    }
}
