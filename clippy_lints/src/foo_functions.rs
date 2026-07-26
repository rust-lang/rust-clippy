use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{Fn, NodeId};
use rustc_ast::visit::FnKind;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for ... (describe what the lint matches).
    ///
    /// ### Why is this bad?
    /// Supply the reason for linting the code.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// // A short example of code that triggers the lint
    /// ```
    ///
    /// Use instead:
    /// ```rust,ignore
    /// // A short example of improved code that doesn't trigger the lint
    /// ```
    #[clippy::version = "1.99.0"]
    pub FOO_FUNCTIONS,
    pedantic,
    "function named `foo`, which is not a descriptive name"
}

declare_lint_pass!(FooFunctions => [FOO_FUNCTIONS]);

impl EarlyLintPass for FooFunctions {
    fn check_fn(&mut self, cx: &EarlyContext<'_>, fn_kind: FnKind<'_>, span: rustc_span::Span, _: NodeId) {
        if is_foo_fn(fn_kind) {
            span_lint_and_help(
                cx,
                FOO_FUNCTIONS,
                span,
                "function named `foo`",
                None,
                "consider using a more meaningful name",
            );
        }
    }
}

fn is_foo_fn(fn_kind: FnKind<'_>) -> bool {
    match fn_kind {
        FnKind::Fn(_, _, Fn { ident, .. }) => ident.name.as_str() == "foo",
        FnKind::Closure(..) => false,
    }
}
