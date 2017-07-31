use syntax::ast::{Item, ItemKind, TyKind, ExprKind};
use rustc::lint::{LintPass, EarlyLintPass, LintArray, EarlyContext};
use utils::span_help_and_lint;

/// **What it does:** Checks for constants with an explicit `'static` lifetime.
///
/// **Why is this bad?** Adding `'static` to every reference can create very complicated types.
///
/// **Known problems:** None.
///
/// **Example:**
/// ```rust
///  const FOO: &'static [(&'static str, &'static str, fn(&Bar) -> bool)] = &[..]
/// ```
/// This code can be rewritten as
/// ```rust
///  const FOO: &[(&str, &str, fn(&Bar) -> bool)] = &[...]
/// ```

declare_lint! {
    pub CONST_STATIC_LIFETIME, 
    Warn,
    "Using explicit `'static` lifetime for constants when elision rules would allow omitting them."
}

pub struct StaticConst;

impl LintPass for StaticConst {
    fn get_lints(&self) -> LintArray {
        lint_array!(CONST_STATIC_LIFETIME)
    }
}

impl EarlyLintPass for StaticConst {
    fn check_item(&mut self, cx: &EarlyContext, item: &Item) {
        // Match only constants...
        if let ItemKind::Const(ref var_type, ref expr) = item.node {
            // ... which are actually variables, not declaration such as #![feature(clippy)] ...
            if let ExprKind::Lit(_) = expr.node {
                // ... which are reference ...
                if let TyKind::Rptr(ref optionnal_lifetime, _) = var_type.node {
                    // ... and have an explicit 'static lifetime ...
                    if let Some(lifetime) = *optionnal_lifetime {
                        if lifetime.ident.name == "'static" {
                            span_help_and_lint(cx,
                                               CONST_STATIC_LIFETIME,
                                               lifetime.span,
                                               "Constants have by default a `'static` lifetime",
                                               "consider removing the `'static`");
                        }
                    }
                }
            }
        }
    }
}
