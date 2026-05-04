use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::higher::If;
use clippy_utils::sugg::Sugg;
use clippy_utils::ty::implements_trait;
use clippy_utils::{eq_expr_value, is_in_const_context, peel_blocks};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;
use rustc_span::sym;

declare_clippy_lint! {
    /// ### What it does
    /// Detects manual implementations of `Ord::min` or `Ord::max` using if-else
    /// expressions and suggests using those methods directly.
    ///
    /// ### Why is this bad?
    /// Using `min` or `max` is shorter, clearer, and avoids explicit control flow.
    ///
    /// ### Known issues
    /// Does not apply to floats because `f32`/`f64` implement `PartialOrd` not
    /// `Ord`, and `f32::min`/`f32::max` have different NaN semantics from a plain
    /// if-else comparison.
    ///
    /// ### Example
    /// ```rust
    /// let a = 3_i32;
    /// let b = 5_i32;
    /// let _ = if a > b { b } else { a };
    /// ```
    /// Use instead:
    /// ```rust
    /// let a = 3_i32;
    /// let b = 5_i32;
    /// let _ = a.min(b);
    /// ```
    #[clippy::version = "1.97.0"]
    pub MANUAL_MIN_MAX,
    complexity,
    "using an if-else instead of `Ord::min` or `Ord::max`"
}

declare_lint_pass!(ManualMinMax => [MANUAL_MIN_MAX]);

impl<'tcx> LateLintPass<'tcx> for ManualMinMax {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() || is_in_const_context(cx) {
            return;
        }
        let Some(If {
            cond,
            then,
            r#else: Some(else_expr),
        }) = If::hir(expr)
        else {
            return;
        };
        let ExprKind::Binary(op, lhs, rhs) = cond.kind else {
            return;
        };
        let then_val = peel_blocks(then);
        let else_val = peel_blocks(else_expr);

        // Determine which method to suggest and which expression is the receiver.
        //
        // The receiver must be the value returned in the equality/tie case, because
        // `x.min(y)` and `x.max(y)` both return `x` when the two values are equal.
        //
        // Pattern table (A = lhs of comparison, B = rhs of comparison):
        //   then=A, else=B, op=<  → B.min(A)  (else=B returned when a==b)
        //   then=A, else=B, op=<= → A.min(B)  (then=A returned when a==b)
        //   then=B, else=A, op=>  → A.min(B)  (else=A returned when a==b)
        //   then=B, else=A, op=>= → B.min(A)  (then=B returned when a==b)
        //   then=A, else=B, op=>  → B.max(A)  (else=B returned when a==b)
        //   then=A, else=B, op=>= → A.max(B)  (then=A returned when a==b)
        //   then=B, else=A, op=<  → A.max(B)  (else=A returned when a==b)
        //   then=B, else=A, op=<= → B.max(A)  (then=B returned when a==b)
        let (method, receiver_expr, arg_expr) = match op.node {
            BinOpKind::Lt => {
                if eq_expr_value(cx, lhs, then_val) && eq_expr_value(cx, rhs, else_val) {
                    ("min", rhs, lhs)
                } else if eq_expr_value(cx, rhs, then_val) && eq_expr_value(cx, lhs, else_val) {
                    ("max", lhs, rhs)
                } else {
                    return;
                }
            },
            BinOpKind::Le => {
                if eq_expr_value(cx, lhs, then_val) && eq_expr_value(cx, rhs, else_val) {
                    ("min", lhs, rhs)
                } else if eq_expr_value(cx, rhs, then_val) && eq_expr_value(cx, lhs, else_val) {
                    ("max", rhs, lhs)
                } else {
                    return;
                }
            },
            BinOpKind::Gt => {
                if eq_expr_value(cx, rhs, then_val) && eq_expr_value(cx, lhs, else_val) {
                    ("min", lhs, rhs)
                } else if eq_expr_value(cx, lhs, then_val) && eq_expr_value(cx, rhs, else_val) {
                    ("max", rhs, lhs)
                } else {
                    return;
                }
            },
            BinOpKind::Ge => {
                if eq_expr_value(cx, rhs, then_val) && eq_expr_value(cx, lhs, else_val) {
                    ("min", rhs, lhs)
                } else if eq_expr_value(cx, lhs, then_val) && eq_expr_value(cx, rhs, else_val) {
                    ("max", lhs, rhs)
                } else {
                    return;
                }
            },
            _ => return,
        };

        // Only Ord types; floats implement PartialOrd but not Ord.
        let ty = cx.typeck_results().expr_ty(receiver_expr);
        let is_ord = cx
            .tcx
            .get_diagnostic_item(sym::Ord)
            .is_some_and(|id| implements_trait(cx, ty, id, &[]));
        if !is_ord {
            return;
        }

        let receiver = Sugg::hir(cx, receiver_expr, "..").maybe_paren();
        let arg = Sugg::hir(cx, arg_expr, "..");
        span_lint_and_sugg(
            cx,
            MANUAL_MIN_MAX,
            expr.span,
            format!("manual implementation of `Ord::{method}`"),
            format!("use `{method}` instead"),
            format!("{receiver}.{method}({arg})"),
            Applicability::MachineApplicable,
        );
    }
}
