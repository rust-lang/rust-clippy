use syntax::ast::{Item, ItemKind, TyKind};
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
///  const GATED_CFGS: &'static [(&'static str, &'static str, fn(&Features) -> bool)] = &[..]
/// ```
/// This code can be rewritten as
/// ```rust
///  const GATED_CFGS: &[(&str, &str, fn(&Features) -> bool)] = &[...]
/// ```
///
///
declare_lint! {
    pub CONST_STATIC_LIFETIME, 
    Warn,
    "Constants have by default a `'static` lifetime ."
}



pub struct StaticConst;


impl LintPass for StaticConst {
    fn get_lints(&self) -> LintArray {
        lint_array!(CONST_STATIC_LIFETIME)
    }
}

impl EarlyLintPass for StaticConst {
    fn check_item(&mut self, cx: &EarlyContext, item: &Item) {

        if let ItemKind::Const(ref p_ty, _) = item.node {

            if let TyKind::Rptr(ref optionnal_lifetime, _) = p_ty.node {
                if let &Some(lifetime) = optionnal_lifetime {
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
