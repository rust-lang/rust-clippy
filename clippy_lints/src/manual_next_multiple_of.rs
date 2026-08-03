use clippy_config::Conf;
use clippy_utils::consts::integer_const;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{DIV_CEIL, Msrv, NEXT_MULTIPLE_OF};
use clippy_utils::res::MaybeDef;
use clippy_utils::source::snippet_with_context;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;
use rustc_span::Symbol;

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
        let (a, b, checked) = match cx.typeck_results().expr_ty(expr).kind() {
            ty::Uint(_) => {
                if let Some((a, b)) = match_arith_pattern(cx, expr) {
                    (a, b, false)
                } else if let Some((a, b)) = match_power_of_two_pattern(cx, expr) {
                    (a, b, false)
                } else if self.msrv.meets(cx, DIV_CEIL)
                    && let Some((a, b)) = match_div_ceil_pattern(cx, expr)
                {
                    (a, b, false)
                } else {
                    return;
                }
            },
            ty::Int(_) => {
                // unstable
                return;
            },
            ty::Adt(def, generic_args)
                if def.is_diag_item(&cx.tcx, sym::Option)
                    && let Some(ty) = generic_args[0].as_type() =>
            {
                match ty.kind() {
                    ty::Uint(_) => {
                        if self.msrv.meets(cx, DIV_CEIL)
                            && let Some((a, b)) = match_div_ceil_pattern_checked(cx, expr)
                        {
                            (a, b, true)
                        } else {
                            return;
                        }
                    },
                    ty::Int(_) => {
                        // unstable
                        return;
                    },
                    _ => return,
                }
            },
            _ => return,
        };

        let mut app = Applicability::MachineApplicable;
        let sugg = {
            let (a, _) = snippet_with_context(cx, a.span, expr.span.ctxt(), "..", &mut app);
            let (b, _) = snippet_with_context(cx, b.span, expr.span.ctxt(), "..", &mut app);

            if checked {
                format!("{a}.checked_next_multiple_of({b})")
            } else {
                format!("{a}.next_multiple_of({b})")
            }
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

// Returns `(a, b)` of `(a + b) & !b` where `b + 1` is a power of two
fn match_power_of_two_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    //  x & y
    let (lhs, rhs) = unpack_bin_op(expr, BinOpKind::BitAnd)?;

    // (a + b) & !c
    let (a, b, c) = if let Some((a, b)) = unpack_bin_op(lhs, BinOpKind::Add)
        && let Some(c) = unpack_un_op(rhs, UnOp::Not)
    {
        (a, b, c)
    } else if let Some((a, b)) = unpack_bin_op(rhs, BinOpKind::Add)
        && let Some(c) = unpack_un_op(lhs, UnOp::Not)
    {
        (a, b, c)
    } else {
        return None;
    };

    // `v = 2^k - 1`
    if integer_const(cx, c, expr.span.ctxt()).is_some_and(|v| v & v.wrapping_add(1) == 0) {
        if eq_expr_value(cx, expr.span.ctxt(), b, c) {
            Some((a, c))
        } else if eq_expr_value(cx, expr.span.ctxt(), a, c) {
            Some((b, c))
        } else {
            None
        }
    } else {
        None
    }
}

// Returns `(a, b)` of `a.div_ceil(b) * b`
fn match_div_ceil_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let Some((lhs, rhs)) = unpack_bin_op(expr, BinOpKind::Mul) {
        if let Some((a, [b])) = unpack_method_call(lhs, sym::div_ceil)
            && eq_expr_value(cx, expr.span.ctxt(), b, rhs)
        {
            Some((a, b))
        } else if let Some((a, [b])) = unpack_method_call(rhs, sym::div_ceil)
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

// Returns `(a, b)` of `a.div_ceil(b).checked_mul(b)`
fn match_div_ceil_pattern_checked<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    // x.checked_mul(y)
    let Some((x, [y])) = unpack_method_call(expr, sym::checked_mul) else {
        return None;
    };

    // a.div_ceil(b).checked_mul(b)
    if let Some((a, [b])) = unpack_method_call(x, sym::div_ceil)
        && eq_expr_value(cx, expr.span.ctxt(), y, b)
    {
        Some((a, b))
    } else
    // b.checked_mul(a.div_ceil(b))
    if let Some((a, [b])) = unpack_method_call(y, sym::div_ceil)
        && eq_expr_value(cx, expr.span.ctxt(), x, b)
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

// Returns `a` of `?a`.
fn unpack_un_op<'tcx>(expr: &'tcx Expr<'tcx>, un_op: UnOp) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::Unary(op, expr) = expr.kind
        && op == un_op
    {
        Some(expr)
    } else {
        None
    }
}

// Returns `(a, [b, ..])` of `a.method(b, ..)`.
fn unpack_method_call<'tcx>(expr: &'tcx Expr<'tcx>, method: Symbol) -> Option<(&'tcx Expr<'tcx>, &'tcx [Expr<'tcx>])> {
    if let ExprKind::MethodCall(path, receiver, args, _) = expr.kind
        && path.ident.name == method
    {
        Some((receiver, args))
    } else {
        None
    }
}
