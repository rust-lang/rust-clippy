use clippy_config::Conf;
use clippy_utils::consts::integer_const;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{HasHirId as _, MaybeDef as _, MaybeResPath as _};
use clippy_utils::source::snippet_with_context;
use clippy_utils::{peel_blocks, sym};
use rustc_ast::LitKind;
use rustc_data_structures::fx::FxHashSet;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Block, Expr, ExprKind, HirId};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_session::impl_lint_pass;

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
    /// let i = 31 - x.highest_one().unwrap();
    /// ```
    #[clippy::version = "1.99.0"]
    pub MANUAL_HIGHEST_ONE,
    nursery,
    "default lint description"
}

impl_lint_pass!(ManualHighestOne => [MANUAL_HIGHEST_ONE]);

pub struct ManualHighestOne {
    msrv: Msrv,
    already_linted: FxHashSet<HirId>,
}

impl ManualHighestOne {
    pub fn new(conf: &Conf) -> Self {
        Self {
            msrv: conf.msrv,
            already_linted: FxHashSet::default(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for ManualHighestOne {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let mid_ty = cx.typeck_results().expr_ty(expr);

        if !self.msrv.meets(cx, msrvs::HIGHEST_ONE) || expr.span.from_expansion() || !mid_ty.is_integral() {
            return;
        }

        if let Some(recv) = extract_recv_from_highest_one_equiv(cx, expr) {
            if self.already_linted.contains(&expr.hir_id()) {
                return;
            }

            let mut app = Applicability::MaybeIncorrect;

            let sugg = {
                let (recv_str, _) = snippet_with_context(cx, recv.span, expr.span.ctxt(), "_", &mut app);
                if cx.typeck_results().expr_ty(recv).is_diag_item(cx, sym::NonZero) {
                    format!("{recv_str}.highest_one()")
                } else {
                    format!("{recv_str}.highest_one().unwrap()")
                }
            };

            span_lint_and_sugg(
                cx,
                MANUAL_HIGHEST_ONE,
                expr.span,
                "manually reimplementing `highest_one`",
                "try",
                sugg,
                app,
            );

            return;
        }

        if let ExprKind::If(cond, true_arm, Some(else_block)) = expr.kind
            && let ExprKind::Block(
                Block {
                    expr: Some(false_arm), ..
                },
                ..,
            ) = else_block.kind
            && let ExprKind::Binary(bin_op, cond_lhs, cond_rhs) = cond.kind
        {
            let check_cond = |lhs, rhs| {
                if Some(0) == extract_lit(lhs) {
                    Some(rhs)
                } else if Some(0) == extract_lit(rhs) {
                    Some(lhs)
                } else {
                    None
                }
            };

            // if x == 0 { .. } else { .. }
            let (recv, nonzero_arm) = if bin_op.node == BinOpKind::Eq
                && let Some(recv) = extract_recv_from_highest_one_equiv(cx, peel_blocks(false_arm))
                && let Some(var) = check_cond(cond_lhs, cond_rhs)
                // check if `var` and `recv` are the same identifier
            && matches!((var.res_local_id(), recv.res_local_id()), (Some(l), Some(r)) if l == r)
            {
                self.already_linted.insert(peel_blocks(false_arm).hir_id());
                (recv, true_arm)
            } else
            // if x != 0 { .. } else { .. }
            if bin_op.node == BinOpKind::Ne
                && let Some(recv) = extract_recv_from_highest_one_equiv(cx, peel_blocks(true_arm))
                && let Some(var) = check_cond(cond_lhs, cond_rhs)
                // check if `var` and `recv` are the same identifier
            && matches!((var.res_local_id(), recv.res_local_id()), (Some(l), Some(r)) if l == r)
            {
                self.already_linted.insert(peel_blocks(true_arm).hir_id());
                (recv, *false_arm)
            } else {
                return;
            };

            let mut app = Applicability::MaybeIncorrect;
            let (recv, _) = snippet_with_context(cx, recv.span, expr.span.ctxt(), "_", &mut app);
            let (nonzero_arm, _) = snippet_with_context(cx, nonzero_arm.span, expr.span.ctxt(), "_", &mut app);

            span_lint_and_sugg(
                cx,
                MANUAL_HIGHEST_ONE,
                expr.span,
                "manually reimplementing `highest_one`",
                "try",
                format!("{recv}.highest_one().unwrap_or({nonzero_arm})"),
                app,
            );
        }
    }
}

/// Returns `x` of `BITS - 1 - x.leading_zeros()`-like expressions.
fn extract_recv_from_highest_one_equiv<'a>(cx: &LateContext<'a>, expr: &'a Expr<'a>) -> Option<&'a Expr<'a>> {
    match expr.kind {
        // BITS - 1 - x.leading_zeros()
        // BITS - 1 - nonzero.get().leading_zeros()
        ExprKind::Binary(bin_op, lhs, rhs) if bin_op.node == BinOpKind::Sub => {
            // literal - x.leading_zeros()
            if let Some(recv) = extract_recv_of_lz(rhs)
                    && let Some(lit) = extract_lit(lhs)
                    // check literal value
                    && let ty = cx.typeck_results().expr_ty(recv)
                    && let Some(bits) = bit_width(cx, ty)
                    && lit == bits.wrapping_sub(1)
            {
                return Some(recv);
            }

            // (BITS - 1) - x.leading_zeros()
            if let ExprKind::Binary(bin_op, inner_lhs, inner_rhs) = lhs.kind
                    && bin_op.node == BinOpKind::Sub
                    && let Some(recv) = check_lit_one_and_extract_recv_of_lz(inner_rhs, rhs)
                    // check BITS
                    && let ty = cx.typeck_results().expr_ty(recv)
                    && integer_const(cx, inner_lhs, expr.span.ctxt()) == bit_width(cx, ty)
            {
                return Some(recv);
            }

            // BITS - (1 + x.leading_zeros())
            if let ExprKind::Binary(bin_op, inner_lhs, inner_rhs) = rhs.kind
                    && bin_op.node == BinOpKind::Add
                    && let Some(recv) = check_lit_one_and_extract_recv_of_lz(inner_lhs, inner_rhs)
                    // check BITS
                    && let ty = cx.typeck_results().expr_ty(recv)
                    && integer_const(cx, lhs, expr.span.ctxt()) == bit_width(cx, ty)
            {
                return Some(recv);
            }
        },
        _ => (),
    }
    None
}

fn extract_recv_of_lz<'a>(expr: &'a Expr<'a>) -> Option<&'a Expr<'a>> {
    if let ExprKind::MethodCall(method, recv, [], _) = expr.kind
        && method.ident.name == sym::leading_zeros
    {
        Some(recv)
    } else {
        None
    }
}

fn extract_lit(expr: &Expr<'_>) -> Option<u128> {
    if let ExprKind::Lit(lit) = expr.kind
        && let LitKind::Int(Pu128(lit), _) = lit.node
    {
        Some(lit)
    } else {
        None
    }
}

fn check_lit_one_and_extract_recv_of_lz<'a>(expr1: &'a Expr<'a>, expr2: &'a Expr<'a>) -> Option<&'a Expr<'a>> {
    if let Some(recv) = extract_recv_of_lz(expr1)
        && Some(1) == extract_lit(expr2)
    {
        Some(recv)
    } else if let Some(recv) = extract_recv_of_lz(expr2)
        && Some(1) == extract_lit(expr1)
    {
        Some(recv)
    } else {
        None
    }
}

fn bit_width(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<u128> {
    match ty.kind() {
        ty::Uint(uint_ty) => uint_ty.bit_width().map(u128::from),
        ty::Int(int_ty) => int_ty.bit_width().map(u128::from),
        ty::Adt(adt_def, args) if adt_def.is_diag_item(cx, sym::NonZero) => bit_width(cx, args[0].expect_ty()),
        _ => None,
    }
}
