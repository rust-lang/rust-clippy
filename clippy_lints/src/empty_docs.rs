use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::*;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    /// Detects documentation that is empty.
    /// ### Why is this bad?
    /// It is unlikely that there is any reason to have empty documentation for an item
    ///
    /// ### Example
    /// ```rust
    /// ///
    /// fn returns_true() {
    ///     true
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// fn returns_true() {
    ///     true
    /// }
    /// ```
    #[clippy::version = "1.74.0"]
    pub EMPTY_DOCS,
    suspicious,
    "docstrings exist but documentation is empty"
}

declare_lint_pass!(EmptyDocs => [EMPTY_DOCS]);

fn trim_comment(comment: &str) -> String {
    comment
        .trim()
        .split("\n")
        .map(|comment| comment.trim().trim_matches('*').trim_matches('!'))
        .collect::<Vec<&str>>()
        .join("")
}

impl EarlyLintPass for EmptyDocs {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attribute: &Attribute) {
        if let AttrKind::DocComment(_line, comment) = attribute.kind {
            if trim_comment(comment.as_str()).len() == 0 {
                span_lint_and_help(
                    cx,
                    EMPTY_DOCS,
                    attribute.span,
                    "empty doc comment",
                    None,
                    "consider removing or fill it",
                );
            }
        }
    }
}
