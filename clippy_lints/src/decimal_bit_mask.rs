use clippy_utils::diagnostics::span_lint;
use clippy_utils::source::snippet_opt;
use rustc_hir::*;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::source_map::Spanned;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for decimal literals used as bit masks in bitwise operations.
    ///
    /// ### Why is this bad?
    /// Using decimal literals for bit masks can make the code less readable and obscure the intended bit pattern.
    /// Binary or hexadecimal literals make the bit pattern more explicit and easier to understand at a glance.
    ///
    /// ### Example
    /// ```rust,no_run
    /// let a = 15 & 6; // Bit pattern is not immediately clear
    /// ```
    /// Use instead:
    /// ```rust,no_run
    /// let a = 0b1111 & 0b0110;
    /// ```
    #[clippy::version = "1.87.0"]
    pub DECIMAL_BIT_MASK,
    nursery,
    "default lint description"
}

declare_lint_pass!(DecimalBitMask => [DECIMAL_BIT_MASK]);

impl<'tcx> LateLintPass<'tcx> for DecimalBitMask {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, e: &'tcx Expr<'_>) {
        if let ExprKind::Binary(
            Spanned {
                node: BinOpKind::BitAnd | BinOpKind::BitOr | BinOpKind::BitXor,
                ..
            },
            Expr {
                kind: kind1,
                span: span1,
                ..
            },
            Expr {
                kind: kind2,
                span: span2,
                ..
            },
        ) = &e.kind
        {
            if let ExprKind::Lit(_) = kind1
                && let Some(snippet) = snippet_opt(cx, *span1)
                && !snippet.starts_with("0b")
                && !snippet.starts_with("0x")
            {
                span_lint(
                    cx,
                    DECIMAL_BIT_MASK,
                    e.span,
                    "Using decimal literal for bit mask. Consider using binary (0b...) or hexadecimal (0x...) notation for better readability.",
                );
            }

            if let ExprKind::Lit(_) = kind2
                && let Some(snippet) = snippet_opt(cx, *span2)
                && !snippet.starts_with("0b")
                && !snippet.starts_with("0x")
            {
                span_lint(
                    cx,
                    DECIMAL_BIT_MASK,
                    e.span,
                    "Using decimal literal for bit mask. Consider using binary (0b...) or hexadecimal (0x...) notation for better readability.",
                );
            }
        }
        if let ExprKind::AssignOp(
            Spanned {
                node: AssignOpKind::BitAndAssign | AssignOpKind::BitOrAssign | AssignOpKind::BitXorAssign,
                ..
            },
            _,
            Expr {
                kind: ExprKind::Lit(_),
                span,
                ..
            },
        ) = &e.kind
        {
            if let Some(snippet) = snippet_opt(cx, *span)
                && !snippet.starts_with("0b")
                && !snippet.starts_with("0x")
            {
                span_lint(
                    cx,
                    DECIMAL_BIT_MASK,
                    e.span,
                    "Using decimal literal for bit mask. Consider using binary (0b...) or hexadecimal (0x...) notation for better readability.",
                );
            }
        }
    }
}
