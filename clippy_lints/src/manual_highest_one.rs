use clippy_config::Conf;
use clippy_utils::comparisons::{Rel, normalize_comparison};
use clippy_utils::consts::{ConstEvalCtxt, Constant, integer_const, is_zero_integer_const};
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{HasHirId as _, MaybeDef as _};
use clippy_utils::source::snippet_with_context;
use clippy_utils::{eq_expr_value, sym};
use rustc_errors::Applicability;
use rustc_hir::{Arm, BinOpKind, Expr, ExprKind, MatchSource, Node, PatKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;
use rustc_span::{Span, Symbol};

declare_clippy_lint! {
    /// ### What it does
    /// Checks for manual implementations of `x.highest_one()`.
    ///
    /// ### Why is this bad?
    /// Manual implementation of `highest_one()` is error-prone and less readable.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u32 = 5;
    /// let i = 31 - x.leading_zeros();
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u32 = 5;
    /// let i = x.highest_one().unwrap();
    /// ```
    #[clippy::version = "1.99.0"]
    pub MANUAL_HIGHEST_ONE,
    nursery,
    "manually reimplementing `highest_one`"
}

impl_lint_pass!(ManualHighestOne => [MANUAL_HIGHEST_ONE]);

pub struct ManualHighestOne {
    msrv: Msrv,
}

impl ManualHighestOne {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv.into() }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualHighestOne {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let mid_ty = cx.typeck_results().expr_ty(expr);

        if !self.msrv.meets(cx, msrvs::HIGHEST_ONE) || expr.span.from_expansion() || !mid_ty.is_integral() {
            return;
        }

        let Some(recv) = extract_recv_from_highest_one_equiv(cx, expr) else {
            return;
        };
        let h1_expr = expr;

        if cx.typeck_results().expr_ty(recv).is_diag_item(cx, sym::NonZero) {
            let mut app = Applicability::MachineApplicable;

            let sugg = {
                let (recv_str, _) = snippet_with_context(cx, recv.span, h1_expr.span.ctxt(), "..", &mut app);

                format!("{recv_str}.highest_one()")
            };

            span_lint_and_sugg(
                cx,
                MANUAL_HIGHEST_ONE,
                h1_expr.span,
                "manually reimplementing `highest_one`",
                "try",
                sugg,
                app,
            );

            return;
        }

        // Walk the node up
        let mut hir_id = h1_expr.hir_id();
        loop {
            match cx.tcx.parent_hir_node(hir_id) {
                Node::Arm(arm) => hir_id = arm.hir_id,
                Node::Block(block)
                    if block.stmts.is_empty()
                // Otherwise, this block may have side effect
                && block.expr.is_some_and(|e| e.hir_id == hir_id) =>
                {
                    hir_id = block.hir_id;
                },
                Node::Expr(expr) => match expr.kind {
                    ExprKind::Block(..) => hir_id = expr.hir_id,
                    // if x == 0 {} else {}
                    // if x != 0 {} else {}
                    // if unsigned > 0 {} else {}
                    ExprKind::If(condition, then_block, Some(else_block))
                        if matches!(else_block.kind, ExprKind::Block(..)) =>
                    {
                        if lint_if_pattern(cx, expr.span, condition, then_block, else_block, h1_expr, recv) {
                            return;
                        }
                        break;
                    },
                    // match u {
                    //     0 => do_something(),
                    //     _ => u.highest_one_like(),
                    // }
                    ExprKind::Match(scrutinee, [arm1, arm2], MatchSource::Normal) => {
                        if lint_match_pattern(cx, expr.span, scrutinee, arm1, arm2, h1_expr, recv) {
                            return;
                        }
                        break;
                    },
                    _ => break,
                },
                _ => break,
            }
        }

        let mut app = Applicability::MachineApplicable;
        let sugg = {
            let (recv_str, _) = snippet_with_context(cx, recv.span, h1_expr.span.ctxt(), "..", &mut app);

            format!("{recv_str}.highest_one().unwrap()")
        };
        span_lint_and_sugg(
            cx,
            MANUAL_HIGHEST_ONE,
            h1_expr.span,
            "manually reimplementing `highest_one`",
            "try",
            sugg,
            app,
        );
    }
}

fn lint_if_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    span: Span,
    condition: &'tcx Expr<'tcx>,
    then_block: &'tcx Expr<'tcx>,
    else_block: &'tcx Expr<'tcx>,
    h1_expr: &'tcx Expr<'tcx>,
    recv_of_h1: &'tcx Expr<'tcx>,
) -> bool {
    // normalize condition: `x ? 0`
    let (rel, value) = {
        if let ExprKind::Binary(bin_op, lhs, rhs) = condition.kind
            && let Some((rel, lhs, rhs)) = normalize_comparison(bin_op.node, lhs, rhs)
        {
            // 0 == x, 0 != x, 0 < x
            if is_zero_integer_const(cx, lhs, condition.span.ctxt()) && !lhs.span.from_expansion() && {
                matches!(rel, Rel::Eq | Rel::Ne) || {
                    matches!(rel, Rel::Lt) && matches!(cx.typeck_results().expr_ty(rhs).kind(), ty::Uint(..))
                }
            } {
                (rel, rhs)
            } else
            // x == 0, x != 0
            if is_zero_integer_const(cx, rhs, condition.span.ctxt())
                && !rhs.span.from_expansion()
                && matches!(rel, Rel::Eq | Rel::Ne)
            {
                (rel, lhs)
            } else {
                return false;
            }
        } else {
            return false;
        }
    };
    // `value` and `recv_of_h1` must indicate the same entity
    if !eq_expr_value(cx, span.ctxt(), value, recv_of_h1) {
        return false;
    }

    // if x == 0 {} else {}
    // if x != 0 {} else {}
    // if 0 < unsigned {} else {}
    let recv_in_then_block = cx.tcx.hir_parent_iter(h1_expr.hir_id).any(|v| v.0 == then_block.hir_id);
    if (rel == Rel::Eq) == recv_in_then_block {
        return false;
    }

    // This lint suggest `unwrap_or_else` to avoid side effect.
    // However, in some simple cases, `unwrap_or` will be better.
    let mut app = Applicability::MaybeIncorrect;

    let sugg = {
        let (recv_str, _) = snippet_with_context(cx, recv_of_h1.span, h1_expr.span.ctxt(), "..", &mut app);
        let (else_str, _) = if recv_in_then_block {
            snippet_with_context(cx, else_block.span, span.ctxt(), "..", &mut app)
        } else {
            snippet_with_context(cx, then_block.span, span.ctxt(), "..", &mut app)
        };

        format!("{recv_str}.highest_one().unwrap_or_else(|| {else_str})")
    };

    span_lint_and_sugg(
        cx,
        MANUAL_HIGHEST_ONE,
        span,
        "manually reimplementing `highest_one`",
        "try",
        sugg,
        app,
    );

    true
}

fn lint_match_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    span: Span,
    scrutinee: &'tcx Expr<'tcx>,
    arm1: &'tcx Arm<'tcx>,
    arm2: &'tcx Arm<'tcx>,
    h1_expr: &'tcx Expr<'tcx>,
    recv_of_h1: &'tcx Expr<'tcx>,
) -> bool {
    if
    // `0` pattern
    arm1.guard.is_none()
        && let PatKind::Expr(pat_expr) = arm1.pat.kind
        && !pat_expr.span.from_expansion()
        && ConstEvalCtxt::new(cx).eval_pat_expr(pat_expr) == Some(Constant::Int(0))
        // `_` pattern
        && arm2.guard.is_none()
        && let PatKind::Wild = arm2.pat.kind
        && arm2.body.hir_id == h1_expr.hir_id
        // same item
        && eq_expr_value(cx, span.ctxt(), scrutinee, recv_of_h1)
    {
        let mut app = Applicability::MaybeIncorrect;

        let sugg = {
            let (recv_str, _) = snippet_with_context(cx, recv_of_h1.span, h1_expr.span.ctxt(), "..", &mut app);
            let (else_str, _) = snippet_with_context(cx, arm1.body.span, span.ctxt(), "..", &mut app);

            format!("{recv_str}.highest_one().unwrap_or_else(|| {else_str})")
        };

        span_lint_and_sugg(
            cx,
            MANUAL_HIGHEST_ONE,
            span,
            "manually reimplementing `highest_one`",
            "try",
            sugg,
            app,
        );

        true
    } else {
        false
    }
}

/// Returns `x` of `BITS - 1 - x.leading_zeros()`-like expressions.
fn extract_recv_from_highest_one_equiv<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // lhs - rhs
    let (lhs, rhs) = unpack_bin_op(expr, BinOpKind::Sub)?;

    // x.bit_width() - 1_u32
    if let Some((recv, [])) = unpack_method_call(lhs, sym::bit_width)
        && integer_const(cx, rhs, expr.span.ctxt()) == Some(1)
    {
        return Some(recv);
    }

    // const - x.leading_zeros()
    if let Some((recv,[])) = unpack_method_call(rhs, sym::leading_zeros)
        && let Some(lit) = integer_const(cx, lhs, expr.span.ctxt())
        // check constant
        && let ty = cx.typeck_results().expr_ty(recv)
        && let Some(bits) = bit_width(cx, ty)
        && lit == bits.wrapping_sub(1)
    {
        return Some(recv);
    }

    // (BITS - 1) - x.leading_zeros()
    if let Some((inner_lhs, inner_rhs)) = unpack_bin_op(lhs, BinOpKind::Sub)
        && let Some(recv) = check_one_and_extract_recv_of_lz(cx, expr,inner_rhs, rhs)
        // check BITS
        && let ty = cx.typeck_results().expr_ty(recv)
        && integer_const(cx, inner_lhs, expr.span.ctxt()) == bit_width(cx, ty)
    {
        return Some(recv);
    }

    // BITS - (1 + x.leading_zeros())
    if let Some(( inner_lhs, inner_rhs)) = unpack_bin_op(rhs, BinOpKind::Add)
        && let Some(recv) = check_one_and_extract_recv_of_lz(cx, expr,inner_lhs, inner_rhs)
        // check BITS
        && let ty = cx.typeck_results().expr_ty(recv)
        && integer_const(cx, lhs, expr.span.ctxt()) == bit_width(cx, ty)
    {
        return Some(recv);
    }

    None
}

fn check_one_and_extract_recv_of_lz<'tcx>(
    cx: &LateContext<'_>,
    h1_expr: &'tcx Expr<'tcx>,
    expr1: &'tcx Expr<'tcx>,
    expr2: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    if let Some((recv, [])) = unpack_method_call(expr1, sym::leading_zeros)
        && integer_const(cx, expr2, h1_expr.span.ctxt()) == Some(1)
    {
        Some(recv)
    } else if let Some((recv, [])) = unpack_method_call(expr2, sym::leading_zeros)
        && integer_const(cx, expr1, h1_expr.span.ctxt()) == Some(1)
    {
        Some(recv)
    } else {
        None
    }
}

fn bit_width<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<u128> {
    match ty.kind() {
        ty::Uint(uint_ty) => uint_ty.bit_width().map(u128::from),
        ty::Int(int_ty) => int_ty.bit_width().map(u128::from),
        ty::Adt(adt_def, args) if adt_def.is_diag_item(cx, sym::NonZero) => bit_width(cx, args[0].expect_ty()),
        _ => None,
    }
}

// Returns `(a, [b, ..])` of `a.method(b, ..)`
fn unpack_method_call<'tcx>(expr: &'tcx Expr<'tcx>, method: Symbol) -> Option<(&'tcx Expr<'tcx>, &'tcx [Expr<'tcx>])> {
    if let ExprKind::MethodCall(path, recv, args, _) = expr.kind
        && path.ident.name == method
    {
        Some((recv, args))
    } else {
        None
    }
}

// Returns `(a, b)` of `a ? b`
fn unpack_bin_op<'tcx>(expr: &'tcx Expr<'tcx>, kind: BinOpKind) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    if let ExprKind::Binary(bin_op, lhs, rhs) = expr.kind
        && bin_op.node == kind
    {
        Some((lhs, rhs))
    } else {
        None
    }
}
