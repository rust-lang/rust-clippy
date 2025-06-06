use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{Pat, PatFieldsRest, PatKind};
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Disallows the use of rest patterns when destructuring structs.
    ///
    /// ### Why is this bad?
    /// It might lead to unhandled fields when the struct changes.
    ///
    /// ### Example
    /// ```no_run
    /// struct S {
    ///     a: u8,
    ///     b: u8,
    ///     c: u8,
    /// }
    ///
    /// let s = S { a: 1, b: 2, c: 3 };
    ///
    /// let S { a, b, .. } = s;
    /// ```
    /// Use instead:
    /// ```no_run
    /// struct S {
    ///     a: u8,
    ///     b: u8,
    ///     c: u8,
    /// }
    ///
    /// let s = S { a: 1, b: 2, c: 3 };
    ///
    /// let S { a, b, c: _ } = s;
    /// ```
    #[clippy::version = "1.89.0"]
    pub REST_WHEN_DESTRUCTURING_STRUCT,
    nursery,
    "rest (..) in destructuring expression"
}
declare_lint_pass!(RestWhenDestructuringStruct => [REST_WHEN_DESTRUCTURING_STRUCT]);

impl EarlyLintPass for RestWhenDestructuringStruct {
    fn check_pat(&mut self, cx: &EarlyContext<'_>, pat: &Pat) {
        if let PatKind::Struct(_, _, _, PatFieldsRest::Rest) = pat.kind {
            span_lint_and_help(
                cx,
                REST_WHEN_DESTRUCTURING_STRUCT,
                pat.span,
                "struct destructuring with rest (..)",
                None,
                "consider explicitly ignoring remaining fields with wildcard patterns (x: _)",
            );
        }
    }
}
