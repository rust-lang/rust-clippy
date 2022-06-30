use rustc_ast::LitKind;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

use clippy_utils::{diagnostics::span_lint_and_then, source::snippet_block};
use rustc_errors::Applicability;

declare_clippy_lint! {
    /// ### What it does
    /// Instead of using an if statement to convert a bool to an int, this lint suggests using a `as` coercion or a `from()` function.
    /// ### Why is this bad?
    /// Coercion or `from()` is idiomatic way to convert bool to a number.
    /// Both methods are guaranteed to return 1 for true, and 0 for false. See https://doc.rust-lang.org/std/primitive.bool.html#impl-From%3Cbool%3E
    ///
    /// ### Example
    /// ```rust
    /// if condition {
    ///     1_i64
    /// } else {
    ///     0
    /// }
    /// ```
    /// Use instead:
    /// ```rust
    /// i64::from(condition)
    /// ```
    /// or
    /// ```rust
    /// condition as i64
    /// ```
    #[clippy::version = "1.64.0"]
    pub BOOL_TO_INT_WITH_IF,
    style,
    "default lint description"
}
declare_lint_pass!(BoolToIntWithIf => [BOOL_TO_INT_WITH_IF]);

impl<'tcx> LateLintPass<'tcx> for BoolToIntWithIf {
    fn check_expr(&mut self, ctx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if !expr.span.from_expansion() {
            check_if_else(ctx, expr);
        }
    }
}

fn check_if_else<'tcx>(ctx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>) {
    if_chain!(
        if let ExprKind::If(check, then, Some(else_)) = &expr.kind;

        if let Some(then_lit) = int_literal(then);
        if let Some(else_lit) = int_literal(else_);

        if check_int_literal_equals_val(then_lit, 1);
        if check_int_literal_equals_val(else_lit, 0);

        then {
            let lit_type = ctx.typeck_results().expr_ty(then_lit); // then and else must be of same type

            span_lint_and_then(ctx, BOOL_TO_INT_WITH_IF, expr.span, "boolean to int conversion using if", |diag| {
                diag.span_suggestion(
                    expr.span,
                    "replace with from",
                    format!(
                        "{}::from({})",
                        lit_type,
                        snippet_block(ctx, check.span, "..", None),
                    ),
                    Applicability::MachineApplicable,
                );

                diag.span_suggestion(
                    expr.span,
                    "replace with coercion",
                    format!(
                        "({}) as {}",
                        snippet_block(ctx, check.span, "..", None),
                        lit_type,
                    ),
                    Applicability::MachineApplicable,
                );
            });
        }
    );
}

// If block contains only a int literal expression, return literal expression
fn int_literal<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>) -> Option<&'tcx rustc_hir::Expr<'tcx>> {
    if_chain!(
        if let ExprKind::Block(block, _) = expr.kind;
        if let Block {
            stmts: [],       // Shouldn't lint if statements with side effects
            expr: Some(expr),
            hir_id: _,
            rules: _,
            span: _,
            targeted_by_break: _,
        } = block;
        if let ExprKind::Lit(lit) = &expr.kind;

        if let LitKind::Int(_, _) = lit.node;

        then {
            return Some(expr)
        }
    );

    None
}

fn check_int_literal_equals_val<'tcx>(expr: &'tcx rustc_hir::Expr<'tcx>, expected_value: u128) -> bool {
    if_chain!(
        if let ExprKind::Lit(lit) = &expr.kind;
        if let LitKind::Int(val, _) = lit.node;
        if val == expected_value;

        then {
            return true;
        }
    );

    return false;
}
