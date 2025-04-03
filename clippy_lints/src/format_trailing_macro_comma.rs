use rustc_ast::ast::*;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    ///
    /// ### Why is this bad?
    ///
    /// ### Example
    /// ```no_run
    /// // example code where clippy issues a warning
    /// ```
    /// Use instead:
    /// ```no_run
    /// // example code which does not raise clippy warning
    /// ```
    #[clippy::version = "1.86.0"]
    pub FORMAT_TRAILING_MACRO_COMMA,
    style,
    "default lint description"
}

declare_lint_pass!(FormatTrailingMacroComma => [FORMAT_TRAILING_MACRO_COMMA]);

impl EarlyLintPass for FormatTrailingMacroComma {}
