use clippy_utils::diagnostics::span_lint_and_sugg;
use rustc_ast::ast::{Expr, ExprKind, LitFloatType, LitKind};
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_clippy_lint! {
    /// ### What it does
    ///
    /// Detects cases where a whole-number literal float is being rounded, using
    /// the `floor`, `ceil`, or `round` methods.
    ///
    /// ### Why is this bad?
    ///
    /// This is unnecessary and confusing to the reader. Doing this is probably a mistake.
    ///
    /// ### Example
    /// ```rust
    /// let x = 1f32.ceil();
    /// ```
    /// Use instead:
    /// ```rust
    /// let x = 1f32;
    /// ```
    #[clippy::version = "1.62.0"]
    pub UNNECESSARY_ROUNDING,
    nursery,
    "Rounding a whole number literal, which is useless"
}
declare_lint_pass!(UnnecessaryRounding => [UNNECESSARY_ROUNDING]);

// TODO also round and float
fn is_useless_ceil(expr: &Expr) -> Option<(String, String)> {
    if let ExprKind::MethodCall(name_ident, args, _) = &expr.kind
        && let method_name = name_ident.ident.name.to_ident_string()
        && (method_name == "ceil" || method_name == "round" || method_name == "floor")
        && !args.is_empty()
        && let ExprKind::Lit(spanned) = &args[0].kind
        && let LitKind::Float(symbol, ty) = spanned.kind {
            let f = symbol.as_str().parse::<f64>().unwrap();
            let f_str = symbol.to_string() + if let LitFloatType::Suffixed(ty) = ty {
                ty.name_str()
            } else {
                ""
            };
            if f.fract() < f64::EPSILON {
                Some((method_name, f_str))
            } else {
                None
            }
        } else {
            None
        }
}

impl EarlyLintPass for UnnecessaryRounding {
    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &Expr) {
        if let Some((method_name, float)) = is_useless_ceil(expr) {
            span_lint_and_sugg(
                cx,
                UNNECESSARY_ROUNDING,
                expr.span,
                &format!("used the `{}` method with a whole number float", method_name),
                &format!("remove the `{}` method call", method_name),
                float,
                Applicability::MachineApplicable,
            );
        }
    }
}
