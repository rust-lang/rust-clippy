use crate::utils::{snippet, span_lint_and_sugg};
use if_chain::if_chain;
use rustc::lint::{EarlyContext, EarlyLintPass, LintArray, LintPass};
use rustc::{declare_tool_lint, lint_array};
use rustc_errors::Applicability;
use syntax::ast::{PatKind, Stmt, StmtKind};

declare_clippy_lint! {
    /// **What is does:** Checks for underscore bindings inside a struct or tuple which could encompass the entire binding.
    /// 
    /// **Why is this bad?** The extra binding information is meaningless and makes the code harder to read.
    /// 
    /// **Known problems:** Won't catch bindings that look like `let (x, (_, _)) = t;`.
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
    "underscore binding occurs inside a struct, but the underscore could be the entire binding"
}

#[derive(Copy, Clone)]
pub struct Pass;

impl LintPass for Pass {
    fn get_lints(&self) -> LintArray {
        lint_array!(SMALL_UNDERSCORE_SCOPE,)
    }

    fn name(&self) -> &'static str {
        "SmallUnderscoreScope"
    }
}

impl EarlyLintPass for Pass {
    fn check_stmt(&mut self, cx: &EarlyContext<'_>, stmt: &Stmt) {
        if_chain! {
            if let StmtKind::Local(ref local) = stmt.node;
            if let PatKind::TupleStruct(_, ref pats, _) | PatKind::Tuple(ref pats, _) = local.pat.node;
            if pats.iter().all(|pat| match pat.node {
                PatKind::Wild => true,
                _ => false,
            });
            then {
                let sugg = if let Some(ref expr) = local.init {
                    format!("let _ = {};", snippet(cx, expr.span, ".."))
                } else {
                    "let _;".to_string()
                };
                span_lint_and_sugg(
                    cx,
                    SMALL_UNDERSCORE_SCOPE,
                    local.span,
                    "this underscore binding could have a wider scope",
                    "try",
                    sugg,
                    Applicability::MachineApplicable
                )
            }
        }
    }
}
