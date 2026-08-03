use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{DIV_CEIL, Msrv, NEXT_MULTIPLE_OF};
use clippy_utils::source::snippet_with_context;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks manual implementation of `next_multiple_of`.
    ///
    /// ### Why is this bad?
    /// This makes code complex and less readable.
    ///
    /// ### Example
    /// ```no_run
    /// let a = 1_u32;
    /// let b = 2_u32;
    ///
    /// let _ = a.div_ceil(b) * b;
    /// let _ = a + (b - a % b) % b;
    /// ```
    /// Use instead:
    /// ```no_run
    /// let a = 1_u32;
    /// let b = 2_u32;
    ///
    /// let _ = a.next_multiple_of(b);
    /// let _ = a.next_multiple_of(b);
    /// ```
    #[clippy::version = "1.99.0"]
    pub MANUAL_NEXT_MULTIPLE_OF,
    complexity,
    "manually reimplementing `next_multiple_of"
}

impl_lint_pass!(ManualNextMultipleOf => [MANUAL_NEXT_MULTIPLE_OF]);

pub struct ManualNextMultipleOf {
    msrv: Msrv,
}

impl ManualNextMultipleOf {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualNextMultipleOf {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() || !self.msrv.meets(cx, NEXT_MULTIPLE_OF) {
            return;
        }

        // find expression equivalent to `a.next_multiple_of(b)`
        let (a, b) = match cx.typeck_results().expr_ty(expr).kind() {
            ty::Uint(_) => {
                if let Some((a, b)) = match_arith_pattern(cx, expr) {
                    (a, b)
                } else if self.msrv.meets(cx, DIV_CEIL)
                    && let Some((a, b)) = match_div_ceil_pattern(cx, expr)
                {
                    (a, b)
                } else {
                    return;
                }
            },
            ty::Int(_) => {
                // unstable
                return;
            },
            _ => return,
        };

        let mut app = Applicability::MachineApplicable;
        let sugg = {
            let (a, _) = snippet_with_context(cx, a.span, expr.span.ctxt(), "..", &mut app);
            let (b, _) = snippet_with_context(cx, b.span, expr.span.ctxt(), "..", &mut app);

            format!("{a}.next_multiple_of({b})")
        };
        span_lint_and_sugg(
            cx,
            MANUAL_NEXT_MULTIPLE_OF,
            expr.span,
            "manually reimplementing `next_multiple_of",
            "try",
            sugg,
            app,
        );
    }
}

// Returns `(a, b)` of `a + (b - a % b) % b`
fn match_arith_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    // `x + y`
    let (x1, y1) = unpack_bin_op(expr, BinOpKind::Add)?;

    // `a + x % b`
    let (a, b, x2) = if let Some((lhs, rhs)) = unpack_bin_op(x1, BinOpKind::Rem) {
        (y1, rhs, lhs)
    } else if let Some((lhs, rhs)) = unpack_bin_op(y1, BinOpKind::Rem) {
        (x1, rhs, lhs)
    } else {
        return None;
    };

    // `b - a % b`
    if let Some((lhs, rhs)) = unpack_bin_op(x2, BinOpKind::Sub)
        && eq_expr_value(cx, expr.span.ctxt(), lhs, b)
        && let Some((lhs, rhs)) = unpack_bin_op(rhs, BinOpKind::Rem)
        && eq_expr_value(cx, expr.span.ctxt(), lhs, a)
        && eq_expr_value(cx, expr.span.ctxt(), rhs, b)
    {
        Some((a, b))
    } else {
        None
    }
}

// Returns `(a, b)` of `a ? b`.
fn unpack_bin_op<'tcx>(expr: &'tcx Expr<'tcx>, bin_op_kind: BinOpKind) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Binary(bin_op, lhs, rhs) = expr.kind
        && bin_op.node == bin_op_kind
    {
        Some((lhs, rhs))
    } else {
        None
    }
}

// Returns `(a, b)` of `a.div_ceil(b) * b`
fn match_div_ceil_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Binary(bin_op, lhs, rhs) = expr.kind
        && bin_op.node == BinOpKind::Mul
    // && let Some((receiver, ))
    {
        if let Some((a, b)) = unpack_div_ceil(lhs)
            && eq_expr_value(cx, expr.span.ctxt(), b, rhs)
        {
            Some((a, b))
        } else if let Some((a, b)) = unpack_div_ceil(rhs)
            && eq_expr_value(cx, expr.span.ctxt(), b, lhs)
        {
            Some((a, b))
        } else {
            None
        }
    } else {
        None
    }
}

// Returns `(a, b)` of `a.div_ceil(b)`.
fn unpack_div_ceil<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::MethodCall(path, receiver, [arg], _) = expr.kind
        && path.ident.name == sym::div_ceil
    {
        Some((receiver, arg))
    } else {
        None
    }
}
