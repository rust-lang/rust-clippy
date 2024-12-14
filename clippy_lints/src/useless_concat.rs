use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::macros::macro_backtrace;
use clippy_utils::source::snippet_opt;
use clippy_utils::{match_def_path, tokenize_with_text};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind};
use rustc_lexer::TokenKind;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks that the `concat!` macro has at least two arguments.
    ///
    /// ### Why is this bad?
    /// If there are less than 2 arguments, then calling the macro is doing nothing.
    ///
    /// ### Example
    /// ```no_run
    /// let x = concat!("a");
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x = "a";
    /// ```
    #[clippy::version = "1.85.0"]
    pub USELESS_CONCAT,
    complexity,
    "checks that the `concat` macro has at least two arguments"
}

declare_lint_pass!(UselessConcat => [USELESS_CONCAT]);

impl LateLintPass<'_> for UselessConcat {
    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        // Check that the expression is generated by a macro.
        if expr.span.from_expansion()
            // Check that it's a string literal.
            && let ExprKind::Lit(lit) = expr.kind
            && let LitKind::Str(_, _) = lit.node
            // Get the direct parent of the expression.
            && let Some(macro_call) = macro_backtrace(expr.span).next()
            // Check if the `concat` macro from the `core` library.
            && match_def_path(cx, macro_call.def_id, &["core", "macros", "builtin", "concat"])
            // We get the original code to parse it.
            && let Some(original_code) = snippet_opt(cx, macro_call.span)
            // This check allows us to ensure that the code snippet:
            // 1. Doesn't come from proc-macro expansion.
            // 2. Doesn't come from foreign macro expansion.
            //
            // It works as follows: if the snippet we get doesn't contain `concat!(`, then it
            // means it's not code written in the current crate so we shouldn't lint.
            && let mut parts = original_code.split('!')
            && parts.next().is_some_and(|p| p.trim() == "concat")
            && parts.next().is_some_and(|p| p.trim().starts_with('('))
        {
            let mut literal = None;
            let mut nb_commas = 0;
            let mut nb_idents = 0;
            for (token_kind, token_s, _) in tokenize_with_text(&original_code) {
                match token_kind {
                    TokenKind::Eof => break,
                    TokenKind::Literal { .. } => {
                        if literal.is_some() {
                            return;
                        }
                        literal = Some(token_s);
                    },
                    TokenKind::Ident => nb_idents += 1,
                    TokenKind::Comma => {
                        nb_commas += 1;
                        if nb_commas > 1 {
                            return;
                        }
                    },
                    // We're inside a macro definition and we are manipulating something we likely
                    // shouldn't, so aborting.
                    TokenKind::Dollar => return,
                    _ => {},
                }
            }
            let literal = match literal {
                Some(lit) => {
                    // Literals can also be number, so we need to check this case too.
                    if lit.starts_with('"') {
                        lit.to_string()
                    } else {
                        format!("\"{lit}\"")
                    }
                },
                None => "\"\"".to_string(),
            };
            // There should always be the ident of the `concat` macro.
            if nb_idents == 1 {
                span_lint_and_sugg(
                    cx,
                    USELESS_CONCAT,
                    macro_call.span,
                    "unneeded use of `concat!` macro",
                    "replace with",
                    literal,
                    Applicability::MachineApplicable,
                );
            }
        }
    }
}
