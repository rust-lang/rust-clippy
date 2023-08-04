use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::NestedMetaItem;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Disallows usage of any conditional compilation that always excludes code from test builds.
    ///
    /// ### Why is this bad?
    ///
    /// Exclude code from tests builds. Is against simplicity in testing, guarding excessive mocking anti-pattern.
    ///
    /// Can show a codebase with a 100% coverage while having untested code.
    ///
    /// ### Example
    /// ```rust
    /// // an important that is a pain to test but not testing it can actually be dangerous
    ///
    /// #[cfg(not(test))]
    /// important_check();
    /// ```
    ///
    /// You should instead don't exclude any specific code from your test builds.
    #[clippy::version = "1.73.0"]
    pub CFG_NOT_TEST,
    restriction,
    "enforce against excluding code from test builds"
}

declare_lint_pass!(CfgNotTest => [CFG_NOT_TEST]);

impl EarlyLintPass for CfgNotTest {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &rustc_ast::Attribute) {
        if let Some(ident) = attr.ident()
            && ident.name == rustc_span::sym::cfg
            && contains_not_test(attr.meta_item_list().as_deref(), false)
        {
            span_lint_and_help(
                cx,
                CFG_NOT_TEST,
                attr.span,
                "code is excluded from test builds",
                None,
                "consider not excluding any code from test builds",
            );
        }
    }
}

fn contains_not_test(list: Option<&[NestedMetaItem]>, not: bool) -> bool {
    list.map_or(not, |list| {
        list.iter().any(|item| {
            item.ident().map_or(false, |ident| match ident.name {
                rustc_span::sym::not => contains_not_test(item.meta_item_list(), !not),
                rustc_span::sym::test => not,
                _ => contains_not_test(item.meta_item_list(), not),
            })
        })
    })
}
