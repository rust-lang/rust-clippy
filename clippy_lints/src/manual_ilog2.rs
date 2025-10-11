use clippy_config::Conf;
use clippy_utils::diagnostics::span_lint_and_sugg;
use clippy_utils::msrvs::{self, Msrv};
use clippy_utils::res::{MaybeDef, MaybeTypeckRes};
use clippy_utils::source::snippet_with_applicability;
use clippy_utils::{is_from_proc_macro, sym};
use rustc_ast::LitKind;
use rustc_data_structures::packed::Pu128;
use rustc_errors::Applicability;
use rustc_hir::{BinOpKind, Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_session::impl_lint_pass;

declare_clippy_lint! {
    /// ### What it does
    /// Checks for expressions like `31 - x.leading_zeros()` or `x.ilog(2)` which are manual
    /// reimplementations of `x.ilog2()`
    ///
    /// ### Why is this bad?
    /// Manual reimplementations of `ilog2` increase code complexity for little benefit.
    ///
    /// ### Example
    /// ```no_run
    /// let x: u32 = 5;
    /// let log = 31 - x.leading_zeros();
    /// let log = x.ilog(2);
    /// ```
    /// Use instead:
    /// ```no_run
    /// let x: u32 = 5;
    /// let log = x.ilog2();
    /// let log = x.ilog2();
    /// ```
    #[clippy::version = "1.92.0"]
    pub MANUAL_ILOG2,
    complexity,
    "manually reimplementing `ilog2`"
}

pub struct ManualIlog2 {
    msrv: Msrv,
}

impl ManualIlog2 {
    pub fn new(conf: &Conf) -> Self {
        Self { msrv: conf.msrv }
    }
}

impl_lint_pass!(ManualIlog2 => [MANUAL_ILOG2]);

impl LateLintPass<'_> for ManualIlog2 {
    fn check_expr<'tcx>(&mut self, cx: &LateContext<'tcx>, expr: &Expr<'tcx>) {
        if expr.span.in_external_macro(cx.sess().source_map()) {
            return;
        }

        match expr.kind {
            // `BIT_WIDTH - 1 - n.leading_zeros()`
            ExprKind::Binary(op, left, right)
                if left.span.eq_ctxt(right.span)
                    && op.node == BinOpKind::Sub
                    && let ExprKind::Lit(lit) = left.kind
                    && let LitKind::Int(Pu128(val), _) = lit.node
                    && let ExprKind::MethodCall(leading_zeros, recv, [], _) = right.kind
                    && leading_zeros.ident.name == sym::leading_zeros
                    // Whether `leading_zeros` is an inherent method, i.e. doesn't come from some unrelated trait
                    && cx.ty_based_def(right).opt_parent(cx).is_impl(cx)
                    && let ty = cx.typeck_results().expr_ty(recv)
                    && let Some(bit_width) = match ty.kind() {
                        ty::Int(int_ty) => int_ty.bit_width(),
                        ty::Uint(uint_ty) => uint_ty.bit_width(),
                        _ => return,
                    }
                    && val == u128::from(bit_width) - 1
                    && self.msrv.meets(cx, msrvs::ILOG2)
                    && !is_from_proc_macro(cx, expr) =>
            {
                emit(cx, recv, expr);
            },

            // `n.ilog(2)`
            ExprKind::MethodCall(ilog, recv, [two], _)
                if expr.span.eq_ctxt(two.span)
                    && ilog.ident.name == sym::ilog
                    // Whether `ilog` is an inherent method, i.e. doesn't come from some unrelated trait
                    && cx.ty_based_def(expr).opt_parent(cx).is_impl(cx)
                    && let ExprKind::Lit(lit) = two.kind
                    && let LitKind::Int(Pu128(2), _) = lit.node
                    && cx.typeck_results().expr_ty(recv).is_integral()
                    && self.msrv.meets(cx, msrvs::ILOG2)
                    && !is_from_proc_macro(cx, expr) =>
            {
                emit(cx, recv, expr);
            },

            _ => {},
        }
    }
}

fn emit(cx: &LateContext<'_>, recv: &Expr<'_>, full_expr: &Expr<'_>) {
    let mut app = Applicability::MachineApplicable;
    let recv = snippet_with_applicability(cx, recv.span, "_", &mut app);
    span_lint_and_sugg(
        cx,
        MANUAL_ILOG2,
        full_expr.span,
        "manually reimplementing `ilog2`",
        "try",
        format!("{recv}.ilog2()"),
        app,
    );
}
