use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::source::SpanRangeExt;
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_hir::{AssignOpKind, BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::Span;
use rustc_span::source_map::Spanned;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for decimal literals used as bit masks in bitwise operations.
    ///
    /// ### Why is this bad?
    /// Using decimal literals for bit masks can make the code less readable and obscure the intended bit pattern.
    /// Binary, hexadecimal, or octal literals make the bit pattern more explicit and easier to understand at a glance.
    ///
    /// ### Example
    /// ```rust,no_run
    /// let a = 14 & 6; // Bit pattern is not immediately clear
    /// ```
    /// Use instead:
    /// ```rust,no_run
    /// let a = 0b1110 & 0b0110;
    /// ```
    #[clippy::version = "1.92.0"]
    pub DECIMAL_BIT_MASK,
    nursery,
    "use binary, hex, or octal literals for bitwise operations"
}

declare_lint_pass!(DecimalBitMask => [DECIMAL_BIT_MASK]);

fn check_bitwise_binary_expr(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::Binary(op, left, right) = &expr.kind
        && matches!(op.node, BinOpKind::BitAnd | BinOpKind::BitOr | BinOpKind::BitXor)
    {
        for expr in [left, right] {
            if let ExprKind::Lit(lit) = &expr.kind
                && is_decimal_number(cx, lit.span)
                && !is_power_of_twoish(lit)
            {
                emit_lint(cx, lit.span);
            }
        }
    }
}

fn check_bitwise_assign_expr(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if let ExprKind::AssignOp(op, _, e) = &expr.kind
        && matches!(
            op.node,
            AssignOpKind::BitAndAssign | AssignOpKind::BitOrAssign | AssignOpKind::BitXorAssign
        )
        && let ExprKind::Lit(lit) = e.kind
        && is_decimal_number(cx, lit.span)
        && !is_power_of_twoish(&lit)
    {
        emit_lint(cx, lit.span);
    }
}

fn is_decimal_number(cx: &LateContext<'_>, span: Span) -> bool {
    span.check_source_text(cx, |src| {
        !(src.starts_with("0b") || src.starts_with("0x") || src.starts_with("0o"))
    })
}

fn is_power_of_twoish(lit: &Spanned<LitKind>) -> bool {
    if let LitKind::Int(Pu128(val), _) = lit.node {
        return val.is_power_of_two() || val.wrapping_add(1).is_power_of_two();
    }
    false
}

fn emit_lint(cx: &LateContext<'_>, span: Span) {
    span_lint_and_help(
        cx,
        DECIMAL_BIT_MASK,
        span,
        "using decimal literal for bitwise operation",
        None,
        "use binary (0b...), hex (0x...), or octal (0o...) notation for better readability",
    );
}

impl<'tcx> LateLintPass<'tcx> for DecimalBitMask {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if expr.span.from_expansion() {
            return;
        }
        check_bitwise_binary_expr(cx, expr);
        check_bitwise_assign_expr(cx, expr);
    }
}
