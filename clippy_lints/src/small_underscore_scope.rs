use crate::utils::span_lint_and_sugg;
use rustc::lint::{EarlyContext, EarlyLintPass, LintArray, LintPass};
use rustc::{declare_lint_pass, declare_tool_lint};
use rustc_errors::Applicability;
use syntax::ast::*;
use syntax::source_map::Span;

declare_clippy_lint! {
    /// **What is does:** Checks for wildcard patterns inside a struct or tuple which could instead encompass the entire pattern.
    ///
    /// **Why is this bad?** The extra binding information is meaningless and makes the code harder to read.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust,ignore
    /// // Bad
    /// let t = (1, 2);
    /// let (_, _) = t;
    ///
    /// // Good
    /// let t = (1, 2);
    /// let _ = t;
    /// ```
    pub SMALL_UNDERSCORE_SCOPE,
    pedantic,
    "wildcard binding occurs inside a struct, but the wildcard could be the entire binding"
}

declare_lint_pass!(SmallUnderscoreScope => [SMALL_UNDERSCORE_SCOPE]);

impl EarlyLintPass for SmallUnderscoreScope {
    fn check_pat(&mut self, cx: &EarlyContext<'_>, pat: &Pat, _: &mut bool) {
        match pat.node {
            PatKind::TupleStruct(_, ref pats, _) | PatKind::Tuple(ref pats, _) => {
                // `Foo(..)` | `(..)` | `Foo(_, _)` | `(_, _)`
                if pats.is_empty()
                    || pats.iter().all(|pat| match pat.node {
                        PatKind::Wild => true,
                        _ => false,
                    })
                {
                    emit_lint(cx, pat.span);
                }
            },
            PatKind::Struct(_, ref pats, _) => {
                // `Bar { .. }`
                // The `Bar { x: _, y: _ }` is covered by `unneeded_field_pattern`, which suggests `Bar { .. }`
                if pats.is_empty() {
                    emit_lint(cx, pat.span);
                }
            },
            _ => (),
        }
    }
}

fn emit_lint(cx: &EarlyContext<'_>, sp: Span) {
    span_lint_and_sugg(
        cx,
        SMALL_UNDERSCORE_SCOPE,
        sp,
        "this wildcard binding could have a wider scope",
        "try",
        "_".to_string(),
        Applicability::MachineApplicable,
    )
}
