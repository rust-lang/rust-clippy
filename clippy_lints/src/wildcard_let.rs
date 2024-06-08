use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::ast::{Local, PatKind};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_middle::lint::in_external_macro;
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// check for `let _ = ...`.
    ///
    /// this may be used by crates that with to force `#[must_use]`
    /// values to actually used, along with `#[forbid(unused_must_use)]`.
    pub WILDCARD_LET,
    restriction,
    "wildcard let"
}
impl_lint_pass!(WildcardLet => [WILDCARD_LET]);

pub struct WildcardLet {}

impl EarlyLintPass for WildcardLet {
    fn check_local(&mut self, cx: &EarlyContext<'_>, local: &Local) {
        let span = local.pat.span;
        if in_external_macro(cx.sess(), span) {
            return;
        }
        if let PatKind::Wild = local.pat.kind {
            span_lint_and_help(
                cx,
                WILDCARD_LET,
                span,
                "wildcard let",
                None,
                "remove this binding or handle the value",
            );
        }
    }
}
