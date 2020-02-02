use rustc_lint::{EarlyLintPass, EarlyContext};
use rustc_session::{declare_lint_pass, declare_tool_lint};
use syntax::ast::*;
use syntax::visit::FnKind;
use rustc_span::Span;
use rustc_errors::DiagnosticBuilder;
use crate::utils::{span_lint_and_then, multispan_sugg};
use if_chain::if_chain;

declare_clippy_lint! {
    /// **What it does:** checks if any fn parameters have been assigned to a local mutable
    /// variable.
    ///
    /// **Why is this bad?** reduces the complexity of the code by removing a redundant local
    /// mutable variable.
    ///
    /// **Known problems:** None.
    ///
    /// **Example:**
    ///
    /// ```rust
    /// fn f(a: Vec<bool>) {
    ///     let mut a = a;
    ///     // rest of code
    /// }
    /// ```
    /// 
    /// could be defined as 
    ///
    /// ```rust
    /// fn f(mut a: Vec<bool>) {
    ///     // rest of code
    /// }
    /// ```
    pub FN_PARAM_REDEF_AS_MUTABLE,
    complexity,
    "local variables that can be eliminated by updating fn params mutability"
}

declare_lint_pass!(FnParamRedefAsMutable => [FN_PARAM_REDEF_AS_MUTABLE]);

impl EarlyLintPass for FnParamRedefAsMutable {
    fn check_fn(&mut self, cx: &EarlyContext<'_>, fn_kind: FnKind<'_>, fn_decl: &FnDecl, span: Span, _: NodeId) {
        if let FnKind::ItemFn(_, _, _, block) | FnKind::Method(_, _, _, block) = fn_kind {
            for stmt in &block.stmts {
                check_statement(cx, fn_decl, span, stmt);
            }
        }
    }
}

fn check_statement(cx: &EarlyContext<'_>, fn_decl: &FnDecl, fn_span: Span, stmt: &Stmt) {
    if_chain! {
        // Check to see if the local variable is defined as mutable
        if let StmtKind::Local(ref local) = stmt.kind;
        if let PatKind::Ident(mode, ..) = local.pat.kind;
        if let BindingMode::ByValue(mutability) = mode;
        if let Mutability::Mut = mutability;

        if let Some(ref expr) = local.init;
        if let ExprKind::Path(_, ref path) = expr.kind;
        if let Some(ref segment) = path.segments.last();
        if let name = segment.ident.name;

        // The path to fn parameters is 1 in length.
        if path.segments.len() == 1;
        then {
            for param in &fn_decl.inputs {
                if_chain! {
                    if let PatKind::Ident(param_mode, ident, ..) = param.pat.kind; 
                    // Make sure they have the same name & it's not mutable
                    if ident.name == name;
                    if let BindingMode::ByValue(param_mut) = param_mode;
                    if let Mutability::Not = param_mut;

                    then {
                        let sugg = |db: &mut DiagnosticBuilder<'_>| {
                            db.span_help(param.span, "consider making this param `mut`");
                            db.span_help(stmt.span, "consider removing this local variable");

                            multispan_sugg(db, "...".to_string(), vec![]);
                        };

                        span_lint_and_then(
                            cx,
                            FN_PARAM_REDEF_AS_MUTABLE,
                            fn_span,
                            "a parameter was redefined as mutable, can be removed",
                            sugg,
                        );
                    }
                }
            }
        }
    }
}
